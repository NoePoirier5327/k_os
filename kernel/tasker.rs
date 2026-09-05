//! Module de gestion globale des processus du système d'exploitation.

mod process_manager;
mod thread_manager;
mod scheduler;
pub mod elf;

use process_manager::ProcessManager;
use thread_manager::ThreadManager;
use process_manager::process::ProcessKind;
use scheduler::Scheduler;
use process_manager::process::PId;
use thread_manager::thread::TId;
use spin::{Once, Mutex};
use alloc::string::String;
use crate::arch::x86_64::gdt;
use crate::arch::x86_64::stack::{KernelStack16Kib, UserStack16Kib, KernelStackAllocator};
use crate::kernel::Kernel;
use crate::kernel::syscalls::set_new_syscall_stack;
use crate::tasker::thread_manager::thread::ThreadState;

/// Unique instance de l'interface de gestion des processus.
static TASKER_INSTANCE: Once<Mutex<Tasker>> = Once::new();

/// Interface de gestion des processus.
/// Il s'agit d'un singleton.
pub struct Tasker {
    process_manager: ProcessManager,
    thread_manager: ThreadManager,
    scheduler: Scheduler,
    kernel_stack_allocator: KernelStackAllocator,
}

impl Tasker {
    /// Initialise l'interface de gestion des processus si ce n'est pas déjà fait.
    pub fn init() {
        TASKER_INSTANCE.call_once(||
            Mutex::new(
                Self {
                    process_manager: ProcessManager::new(),
                    thread_manager: ThreadManager::new(),
                    scheduler: Scheduler::new(),
                    kernel_stack_allocator: KernelStackAllocator::new()
                }
            )
        );
    }

    /// Interface d'accès à l'instance interne de tasking.
    /// Gère automatiquement la durée de validité du mutex.
    /// Desactive les interruptions le temps de la commande.
    pub fn on_instance<R>(f: impl FnOnce(&mut Tasker) -> R) -> R {
        let tasking = TASKER_INSTANCE.get().expect("Tasking not initialized.");
        x86_64::instructions::interrupts::without_interrupts(|| f(&mut tasking.lock()))
    }

    /// Créer un nouveau processus kernel et renvoie son identifiant.
    /// Alloue automatiquement un thread kernel au nouveau processus.
    ///
    /// # Arguments
    /// * `name`: nom du nouveau processus.
    /// * `entry_point`: point d'entrée du thread attâché au processus.
    ///
    /// # Return
    /// Identifiant du nouveau processus.
    pub fn create_kernel_process(&mut self, name: impl Into<String>, entry_point: u64) -> TaskerResult<PId> {
        // On alloue le nouveau processus
        let pid = self.process_manager.create_kernel_process(name);

        // On alloue la nouvelle pile du thread enfant au nouveau processus.
        let mut kernel_mapper = Kernel::on_instance().mapper();
        let top_vaddr = self.kernel_stack_allocator.allocate_top();
        let kernel_stack = match unsafe { KernelStack16Kib::allocate(&mut kernel_mapper, top_vaddr) } {
            Ok(stack) => stack,
            Err(e) => {
                self.kernel_stack_allocator.deallocate_top(top_vaddr);
                return Err(e);
            }
        };

        // On alloue le thread enfant.
        let tid = self.thread_manager.create_kernel_thread(pid, entry_point, kernel_stack);

        // On l'ajoute au processus parent et à l'ordonnanceur.
        let _ = self.process_manager.get_mut(pid)?.add_thread(tid);
        let _ = self.scheduler.add_thread(tid);

        // On renvoie l'identifiant du nouveau processus.
        Ok(pid)
    }

    /// Créer un nouveau processus utilisateur et renvoie son identifiant.
    /// Alloue un obligatoirement un thread utilisateur enfant.
    /// Cette méthode sert à charger des processus issues de binaire elf
    ///
    /// # Arguments
    /// * `name`: nom du nouveau processus.
    /// * `elf_bytes`: contenu de l'executable binaire sur lequel lancer le nouveau thread.
    pub fn create_user_process(
        &mut self,
        name: impl Into<String>,
        elf_bytes: &[u8],
        ) -> TaskerResult<PId> {
        // On alloue un nouveau processus.
        let pid = self.process_manager.create_user_process(name);
        let process = self.process_manager.get_mut(pid)?;

        // On créer le mapper utilisateur associé au nouveau processus.
        let mut user_mapper = unsafe { process.get_address_space().mapper() };

        // On parse le binaire elf en entrée.
        let entry_point = unsafe { elf::load_elf(elf_bytes, &mut user_mapper) };

        // On alloue la pile kernel du thread enfant.
        let mut kernel_mapper = Kernel::on_instance().mapper();
        let kernel_top_vaddr = self.kernel_stack_allocator.allocate_top();
        let kernel_stack = match unsafe { KernelStack16Kib::allocate(&mut kernel_mapper, kernel_top_vaddr) } {
            Ok(stack) => stack,
            Err(e) => {
                self.kernel_stack_allocator.deallocate_top(kernel_top_vaddr);
                return Err(e)
            }
        };

        // On alloue le haut de pile pour le nouveau thread utilisateur.
        let user_top_vaddr = process.allocate_top_vaddr()?;

        // On alloue la pile utilisateur du thread enfant.
        let user_stack = match unsafe { UserStack16Kib::allocate(&mut user_mapper, user_top_vaddr) } {
            Ok(stack) => stack,
            Err(e) => {
                process.deallocate_top_vaddr(user_top_vaddr).ok();
                return Err(e)
            }
        };

        // On alloue le thread enfant
        let tid = self.thread_manager.create_user_thread(pid, entry_point.as_u64(), user_stack, kernel_stack);

        // On l'ajoute à l'ordonnanceur et à son processus parent.
        self.scheduler.add_thread(tid)?;
        process.add_thread(tid)?;

        Ok(pid)
    }

    /// Créer un nouveau thread kernel et l'associe avec son processus parent.
    ///
    /// # Arguments
    /// * `parent_pid`: identifiant du processus parent au nouveau thread.
    /// * `entry_point`: point d'entré pour l'exécution du nouveau thread.
    ///
    /// # Return
    /// Si tout va bien, renvoie l'identifiant du nouveau thread.
    /// Sinon, si processus parent inaccessible ou pas du type kernel, renvoie une erreur.
    pub fn create_kernel_thread(
        &mut self,
        parent_pid: PId,
        entry_point: u64,
    ) -> TaskerResult<TId> {
        let process = self.process_manager.get(parent_pid)?;
        if process.get_kind() == ProcessKind::User {
            return Err(TaskerError::WrongProcessKind);
        }

        // On alloue la pile kernel pour le nouveau thread.
        let mut kernel_mapper = Kernel::on_instance().mapper();
        let kernel_top_vaddr = self.kernel_stack_allocator.allocate_top();
        let kernel_stack = match unsafe { KernelStack16Kib::allocate(&mut kernel_mapper, kernel_top_vaddr) } {
            Ok(stack) => stack,
            Err(e) => {
                self.kernel_stack_allocator.deallocate_top(kernel_top_vaddr);
                return Err(e)
            }
        };

        // On alloue le nouveau thread
        let tid = self.thread_manager.create_kernel_thread(parent_pid, entry_point, kernel_stack);

        // On l'ajoute au processus parent et au scheduler.
        self.process_manager.add_thread(parent_pid, tid)?;
        self.scheduler.add_thread(tid)?;

        Ok(tid)
    }

    /// Créer un nouveau thread utilisateur et l'associe avec son processus parent.
    ///
    /// # Arguments
    /// * `parent_pid`: identifiant du processus parent au nouveau thread.
    /// * `entry_point`: point d'entré pour l'exécution du nouveau thread.
    ///
    /// # Return
    /// Si tout va bien, renvoie l'identifiant du nouveau thread.
    /// Sinon, si processus parent inaccessible ou pas du type utilisateur, renvoie une erreur.
    pub fn create_user_thread(
        &mut self,
        parent_pid: PId,
        entry_point: u64,
    ) -> TaskerResult<TId> {
        let process = self.process_manager.get_mut(parent_pid)?;
        if process.get_kind() == ProcessKind::Kernel {
            return Err(TaskerError::WrongProcessKind);
        }

        let mut kernel_mapper = Kernel::on_instance().mapper();
        let kernel_top_vaddr = self.kernel_stack_allocator.allocate_top();
        let kernel_stack = match unsafe { KernelStack16Kib::allocate(&mut kernel_mapper, kernel_top_vaddr) } {
            Ok(stack) => stack,
            Err(e) => {
                self.kernel_stack_allocator.deallocate_top(kernel_top_vaddr);
                return Err(e)
            }
        };

        let mut user_mapper = unsafe { process.get_address_space().mapper() };
        let user_top_vaddr = process.allocate_top_vaddr()?;
        let user_stack = match unsafe { UserStack16Kib::allocate(&mut user_mapper, user_top_vaddr) } {
            Ok(stack) => stack,
            Err(e) => {
                process.deallocate_top_vaddr(user_top_vaddr).ok();
                return Err(e)
            }
        };

        let tid = self.thread_manager.create_user_thread(parent_pid, entry_point, user_stack, kernel_stack);
        self.process_manager.add_thread(parent_pid, tid)?;
        self.scheduler.add_thread(tid)?;

        Ok(tid)
    }

    /// Détruis le processus associé à l'identifiant en paramètre ainsi que tous ses threads associés.
    /// Renvoie une erreur si le processus est introuvable.
    pub fn destroy_process(&mut self, pid: PId) -> TaskerResult<()> {
        // On détruit les threads auquel il est associé.
        let process = self.process_manager.get_mut(pid)?;
        process.kill();

        let tids = process.get_threads().clone();
        for tid in tids {
            self.destroy_thread_inner(tid, pid).ok();
        }

        // puis, on le détruit
        self.process_manager.destroy(pid)?;
        Ok(())
    }

    /// Détruis le thread associé à l'identifiant en paramètre, le retire aussi de son processus parent.
    /// Renvoie une erreur si le thread est introuvable.
    pub fn destroy_thread(&mut self, tid: TId) -> TaskerResult<()> {
        let parent_pid = self.thread_manager.get(tid)?.get_parent_pid();
        self.destroy_thread_inner(tid, parent_pid).ok();
        Ok(())
    }

    /// Détruis un thread en prenant en paramètre son processus parent.
    /// On ne fait pas trop attention aux erreurs car le but est de déallouer la mémoire.
    /// Algorithme "best-effort"
    fn destroy_thread_inner(&mut self, tid: TId, pid: PId) -> TaskerResult<()> {
        let process = match self.process_manager.get_mut(pid) {
            Ok(process) => process,

            // S'il y a une erreur à la recupération du processus, on reviens à l'appelant.
            _ => return Ok(())
        };

        // On le retire de son thread parent et de l'ordonnanceur
        self.scheduler.remove_thread(tid).ok();
        process.remove_thread(tid).ok();

        if let Ok(thread) = self.thread_manager.get_mut(tid) {
            thread.kill();
            
            // On désalloue sa pile kernel d'abord.
            let top_vaddr = thread.get_kernel_top_vaddr();
            self.kernel_stack_allocator.deallocate_top(top_vaddr);
            thread.deallocate_kernel_stack();

            // Puis si le processus et lui sont de type utilisateur
            // on désalloue la pile utilisateur.
            if process.get_kind() == ProcessKind::User && thread.get_parent_pid() == pid {
                if let Ok(top_vaddr) = thread.get_user_stack_top_vaddr() {
                    process.deallocate_top_vaddr(top_vaddr).ok();
                }

                let mut user_mapper = unsafe { process.get_address_space().mapper() };
                thread.deallocate_user_stack(&mut user_mapper).ok();
            }
        }

        // enfin, on libère la mémoire du thread.
        self.thread_manager.destroy(tid).ok();

        Ok(())
    }

    /// Fonction de changement de contexte entre deux threads.
    ///
    /// # Argument
    /// * `old_rsp`: pointeur de pile du thread sortant.
    ///
    /// # Return
    /// Pointeur de pile du nouveau thread à s'exécuter.
    pub extern "C" fn handle_switch(old_rsp: u64) -> u64 {
        Tasker::on_instance(|tasker| {
            // Sauvegarde du RSP dans le thread sortant
            if let Some(current_tid) = tasker.scheduler.get_current() {
                if let Ok(thread) = tasker.thread_manager.get_mut(current_tid) {
                    thread.rsp = old_rsp;

                    // On remet le thread sortant dans la file d'attente de l'ordonnanceur
                    // s'il est en état de tourner ou est prêt.
                    if thread.state == ThreadState::Ready || thread.state == ThreadState::Running {
                        // On le remet à prêt tant qu'il n'est pas courant.
                        thread.state = ThreadState::Ready;
                        tasker.scheduler.add_thread(current_tid).ok();
                    }
                }
            }

            // Sélection du thread entrant
            if let Some(next_tid) = tasker.scheduler.pick_next() {
                if let Ok(next_thread) = tasker.thread_manager.get_mut(next_tid) {
                    // On change l'état du nouveau thread à running.
                    next_thread.state = ThreadState::Running;

                    // On met à jour le registre cr3 si nécessaire.
                    let parent_pid = next_thread.get_parent_pid();
                    if let Ok(process) = tasker.process_manager.get(parent_pid) {
                        unsafe { process.get_address_space().swap_pml4() };
                    }

                    // On met à jour la tss et la pile d'appels système.
                    gdt::set_tss_rsp0(next_thread.get_kernel_stack_top());
                    unsafe { set_new_syscall_stack(next_thread.get_kernel_stack_top()); }

                    return next_thread.rsp;
                }
            }

            // Si aucun thread à exécuter, on conserve l'actuel
            old_rsp
        })
    }
}

/// Type centralisant les erreurs de l'interface de gestion des processus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskerError {
    /// On ne trouve pas de processus avec l'identifiant en paramètre.
    ProcessNotFound(PId),

    /// On ne trouve pas de thread avec l'identifiant en paramètre.
    ThreadNotFound(TId),

    /// On manipule un processus n'ayant pas le type auquel on s'attendait.
    WrongProcessKind,

    /// On essaie d'ajouter un élément qui existe déjà.
    AlreadyExists,

    /// Plus assez de mémoire pour l'allocation de pile.
    OutOfMemory,

    /// Impossible de mapper une frame dans un mapper utilisateur.
    MappingFailed,

    /// Signale une addresse mal alignée ou inaccessible.
    UnalignedAddress,
}

/// Interface de manipulation des resultats pouvant renvoyer des Result.
pub type TaskerResult<T> = Result<T, TaskerError>;
