//! Module de gestion globale des processus du système d'exploitation.

mod process_manager;
mod thread_manager;
mod scheduler;

use process_manager::ProcessManager;
use thread_manager::ThreadManager;
use process_manager::process::ProcessKind;
use scheduler::Scheduler;
use process_manager::process::PId;
use thread_manager::thread::TId;
use spin::{Once, Mutex};
use alloc::string::String;

/// Unique instance de l'interface de gestion des processus.
static TASKER_INSTANCE: Once<Mutex<Tasker>> = Once::new();

/// Interface de gestion des processus.
/// Il s'agit d'un singleton.
pub struct Tasker {
    process_manager: ProcessManager,
    thread_manager: ThreadManager,
    scheduler: Scheduler
}

impl Tasker {
    /// Initialise l'interface de gestion des processus si ce n'est pas déjà fait.
    pub fn init() {
        TASKER_INSTANCE.call_once(||
            Mutex::new(
                Self {
                    process_manager: ProcessManager::new(),
                    thread_manager: ThreadManager::new(),
                    scheduler: Scheduler::new()
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
    pub fn create_kernel_process(&mut self, name: impl Into<String>) -> PId {
        self.process_manager.create_kernel_process(name)
    }

    /// Créer un nouveau processus utilisateur et renvoie son identifiant.
    pub fn create_user_process(&mut self, name: impl Into<String>) -> PId {
        self.process_manager.create_user_process(name)
    }

    /// Créer un nouveau thread kernel et l'associe avec son processus parent.
    ///
    /// # Arguments
    /// * `parent_pid`: identifiant du processus parent au nouveau thread.
    /// * `entry`: point d'entré pour l'exécution du nouveau thread.
    /// * `kernel_stack_top`: haut de la pile kernel allouée au nouveau thread.
    ///
    /// # Return
    /// Si tout va bien, renvoie l'identifiant du nouveau thread.
    /// Sinon, si processus parent inaccessible ou pas du type kernel, renvoie une erreur.
    pub fn create_kernel_thread(
        &mut self,
        parent_pid: PId,
        entry: u64,
        kernel_stack_top: u64
    ) -> TaskerResult<TId> {
        let process = self.process_manager.get(parent_pid)?;
        if process.get_kind() == ProcessKind::User {
            return Err(TaskerError::WrongProcessKind);
        }

        let tid = self.thread_manager.create_kernel_thread(parent_pid, entry, kernel_stack_top);
        self.process_manager.add_thread(parent_pid, tid)?;
        self.scheduler.add_thread(tid)?;

        Ok(tid)
    }

    /// Créer un nouveau thread utilisateur et l'associe avec son processus parent.
    ///
    /// # Arguments
    /// * `parent_pid`: identifiant du processus parent au nouveau thread.
    /// * `entry`: point d'entré pour l'exécution du nouveau thread.
    /// * `user_stack_top`: hauteur de la pile utilisateur allouée au nouveau thread.
    /// * `kernel_stack_top`: haut de la pile kernel allouée au nouveau thread.
    ///
    /// # Return
    /// Si tout va bien, renvoie l'identifiant du nouveau thread.
    /// Sinon, si processus parent inaccessible ou pas du type utilisateur, renvoie une erreur.
    pub fn create_user_thread(
        &mut self,
        parent_pid: PId,
        entry: u64,
        user_stack_top: u64,
        kernel_stack_top: u64
    ) -> TaskerResult<TId> {
        let process = self.process_manager.get(parent_pid)?;
        if process.get_kind() == ProcessKind::Kernel {
            return Err(TaskerError::WrongProcessKind);
        }

        let tid = self.thread_manager.create_user_thread(parent_pid, entry, user_stack_top, kernel_stack_top);
        self.process_manager.add_thread(parent_pid, tid)?;
        self.scheduler.add_thread(tid)?;

        Ok(tid)
    }

    /// Détruis le processus associé à l'identifiant en paramètre ainsi que tous ses threads associés.
    /// Renvoie une erreur si le processus est introuvable.
    pub fn destroy_process(&mut self, pid: PId) -> TaskerResult<()> {
        // On détruit les threads auquel il est associé.
        self.process_manager.get_mut(pid)?.kill(); // On marque le processus courant comme mort pour
                                                   // eviter qu'il tourne pendant qu'on le détruit.
        let tids = self.process_manager.get(pid)?.get_threads().clone();
        for tid in tids {
            // On ignore l'erreur de non existence lors de la suppression.
            if let Ok(thread) = self.thread_manager.get_mut(tid) {
                thread.kill();
            }
            self.thread_manager.destroy(tid).ok();
            self.scheduler.remove_thread(tid).ok();
        }

        // puis, on le détruit
        self.process_manager.destroy(pid)?;
        Ok(())
    }

    /// Détruis le thread associé à l'identifiant en paramètre, le retire aussi de son processus parent.
    /// Renvoie une erreur si le thread est introuvable.
    pub fn destroy_thread(&mut self, tid: TId) -> TaskerResult<()> {
        let parent_pid = self.thread_manager.get(tid)?.get_parent_pid();
        self.process_manager.get_mut(parent_pid)?.remove_thread(tid)?;
        if let Ok(thread) = self.thread_manager.get_mut(tid) {
            thread.kill();
        }
        self.thread_manager.destroy(tid).ok();
        self.scheduler.remove_thread(tid).ok();
        Ok(())
    }
}

/// Type centralisant les erreurs de l'interface de gestion des processus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskerError {
    /// On ne trouve pas de processus avec l'identifiant en paramètre.
    ProcessNotFound(PId),

    /// On ne trouve pas de thread avec l'identifiant en paramètre.
    ThreadNotFound(TId),

    /// On manipule deux processus n'ayant pas le même type.
    WrongProcessKind,

     /// On essaie d'ajouter un élément qui existe déjà.
    AlreadyExists,
}

/// Interface de manipulation des resultats pouvant renvoyer des Result.
type TaskerResult<T> = Result<T, TaskerError>;
