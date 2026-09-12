//! Module de gestion globale des processus du système d'exploitation.
// TODO Implémenter des piles à taille variable.

mod process_manager;
mod thread_manager;
mod scheduler;

use process_manager::ProcessManager;
use thread_manager::ThreadManager;
use scheduler::Scheduler;
use process_manager::process::{PId, ProcessKind};
use thread_manager::thread::{TId, ThreadState};
use spin::{Once, Mutex};
use alloc::string::String;
use crate::arch::without_interrupts;
use crate::memory::stack::{KernelStack16Kib, KernelStackAllocator, UserStack16Kib};

/// Unique instance de l'interface de gestion des processus.
static TASKER_INSTANCE: Once<Mutex<Tasker>> = Once::new();

/// Interface de gestion des processus.
/// Il s'agit d'un singleton.
pub struct Tasker {
    process_manager: ProcessManager<'static>,
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
        without_interrupts(|| f(&mut tasking.lock()))
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

        // On lui alloue un thread enfant chargé sur le point d'entré en paramètre.
        self.create_kernel_thread(pid, entry_point)?;

        // On renvoie l'identifiant du nouveau processus.
        Ok(pid)
    }

    /// Créer un nouveau processus utilisateur basé sur un programme en elf64
    /// Alloue un obligatoirement un thread utilisateur enfant.
    /// Cette méthode sert à charger des processus issues de binaire elf
    ///
    /// # Arguments
    /// * `name`: nom du nouveau processus.
    /// * `user_stack_size`: taille de la pile utilisateur allouée au nouveau processus.
    /// * `elf_bytes`: contenu de l'executable binaire sur lequel lancer le nouveau thread.
    pub fn create_user_process_with_elf_entry(
        &mut self,
        name: impl Into<String>,
        user_stack_size: usize,
        elf_bytes: &[u8],
    ) -> TaskerResult<PId> {
        // On alloue un nouveau processus.
        let pid = self.process_manager.create_user_process(name);
        let user_mapper = self.process_manager.get_mut(pid)?.get_user_mapper()?;

        // On parse le elf
        let elf_entry_point = unsafe { crate::fs::elf::load_elf(elf_bytes, user_mapper) };

        // puis, on alloue l'enfant du nouveau processus sur le point d'entré du elf.
        self.create_user_thread(pid, user_stack_size, elf_entry_point.as_u64())?;

        // Et on renvoie son identifiant.
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
        let kernel_top_vaddr = self.kernel_stack_allocator.allocate_top();
        let kernel_stack = match unsafe { KernelStack16Kib::allocate(kernel_top_vaddr) } {
            Ok(stack) => stack,
            Err(e) => {
                self.kernel_stack_allocator.deallocate_top(kernel_top_vaddr);
                panic!("{:?}", e);
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
    /// * `user_stack_size`: taille de la pile utilisateur allouée au nouveau thread.
    /// * `entry_point`: point d'entré pour l'exécution du nouveau thread.
    ///
    /// # Return
    /// Si tout va bien, renvoie l'identifiant du nouveau thread.
    /// Sinon, si processus parent inaccessible ou pas du type utilisateur, renvoie une erreur.
    // NOTE L'argument user_stack_size ne sert à rien car pas de pile à taille variable implémenté.
    pub fn create_user_thread(
        &mut self,
        parent_pid: PId,
        user_stack_size: usize,
        entry_point: u64,
    ) -> TaskerResult<TId> {
        let process = self.process_manager.get_mut(parent_pid)?;
        if process.get_kind() == ProcessKind::Kernel {
            return Err(TaskerError::WrongProcessKind);
        }

        let kernel_top_vaddr = self.kernel_stack_allocator.allocate_top();
        let kernel_stack = match unsafe { KernelStack16Kib::allocate(kernel_top_vaddr) } {
            Ok(stack) => stack,
            Err(e) => {
                self.kernel_stack_allocator.deallocate_top(kernel_top_vaddr);
                panic!("{:?}", e);
            }
        };

        let user_stack_top = process.allocate_user_stack_top()?;

        // On limite la portée de user_mapper pour assurer à rust que l'emprunt est toujours valide.
        let user_stack_allocation_result = {
            let user_mapper = process.get_user_mapper()?;
            unsafe { UserStack16Kib::allocate(user_mapper, user_stack_top) }
        };

        let user_stack = match user_stack_allocation_result {
            Ok(stack) => stack,
            Err(e) => {
                let _ = process.deallocate_user_stack_top(user_stack_top);
                panic!("{:?}", e);
            }
        };

        let tid = self.thread_manager.create_user_thread(parent_pid, entry_point, user_stack_top, kernel_stack, user_stack);
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
            
            // On désalloue sa pile kernel.
            let top_vaddr = thread.get_kernel_top_vaddr();
            self.kernel_stack_allocator.deallocate_top(top_vaddr);
            thread.deallocate_kernel_stack();

            // Puis, on désalloue sa pile utilisateur si besoin.
            if thread.get_parent_pid() == pid && process.get_kind() == ProcessKind::User {
                // On sait, par le test précédent, que process est de type utilisateur et que thread
                // et son enfant, normalement, il est aussi de type utilisateur, donc le unwrap
                // n'est pas dangereux.
                let user_stack_top = thread.get_user_stack_top_vaddr().unwrap();
                process.deallocate_user_stack_top(user_stack_top).unwrap();

                let user_mapper = process.get_user_mapper().unwrap();
                thread.deallocate_user_stack(user_mapper).ok();
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
        use crate::arch::{CPU_CONTEXT, SYSCALL_INTERFACE};
        use crate::arch::hal::cpu::CpuContext;
        use crate::arch::hal::syscalls::SyscallInterface;

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
                    if let Ok(process) = tasker.process_manager.get_mut(parent_pid) {
                        // Si le processus courant est du type utilisateur, alors on met son mapper
                        // comme courant
                        // Sinon, on ne fait rien car le mapping kernel est déjà chargé dans celle
                        // de l'utilisateur.
                        if process.get_kind() == ProcessKind::User {
                            unsafe { process.get_user_mapper().unwrap().set_as_current(); };
                        }
                    }

                    // On met à jour la pile d'exécution noyau.
                    CPU_CONTEXT.update_kernel_stack(next_thread.get_kernel_stack_top());

                    // Puis celle des appels système.
                    SYSCALL_INTERFACE.update_kernel_stack(next_thread.get_kernel_stack_top());

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
}

/// Interface de manipulation des resultats pouvant renvoyer des Result.
pub type TaskerResult<T> = Result<T, TaskerError>;
