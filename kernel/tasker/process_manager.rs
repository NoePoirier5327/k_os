//! Module de gestion de processus kernel et utilisateur.

pub mod process;

use alloc::{collections::btree_map::BTreeMap, string::String};
use process::{PId, Process};
use super::thread_manager::thread::TId;
use crate::tasker::{TaskerError, TaskerResult};

/// Gestionnaire de processus.
/// S'occupe des les créers et des les détruires.
/// L'ordonnanceur, lui, s'occupera des changements de context d'exécution.
pub struct ProcessManager {
    processes: BTreeMap<PId, Process>,
}

impl ProcessManager {
    /// Instancie un nouveau gestionnaire de processus.
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new()
        }
    }

    /// Instancie un nouveau processus kernel.
    ///
    /// # Argument
    /// * `name`: nom du nouveau processus à instancier
    ///
    /// # Return
    /// Identifiant du nouveau processus kernel.
    pub fn create_kernel_process(&mut self, name: impl Into<String>) -> PId {
        let process = Process::new_kernel(name);
        let pid = process.get_pid();
        self.processes.insert(pid, process);
        pid
    }

    /// Instancie un nouveau processus utilisateur.
    ///
    /// # Argument
    /// * `name`: nom du nouveau processus à instancier.
    ///
    /// # Return
    /// Identifiant du nouveau processus utilisateur.
    pub fn create_user_process(&mut self, name: impl Into<String>) -> PId {
        let process = Process::new_user(name);
        let pid = process.get_pid();
        self.processes.insert(pid, process);
        pid
    }

    /// Renvoie, si trouver, un emprunt vers le processus associé à l'identifiant en paramètre.
    pub fn get(&self, pid: PId) -> TaskerResult<&Process> {
        if let Some(process) = self.processes.get(&pid) {
            return Ok(process);
        }

        Err(TaskerError::ProcessNotFound(pid))
    }

    /// Renvoie, si trouver, un emprunt mutable vers le processus associé à l'identifiant en
    /// paramètre.
    pub fn get_mut(&mut self, pid: PId) -> TaskerResult<&mut Process> {
        if let Some(process) = self.processes.get_mut(&pid) {
            return Ok(process);
        }

        Err(TaskerError::ProcessNotFound(pid))
    }

    /// Détruis le processus interne associé à l'identifiant en paramètre.
    /// Renvoie une erreur si le processus à détruire n'existe pas.
    pub fn destroy(&mut self, pid: PId) -> TaskerResult<()> {
        if self.processes.remove(&pid).is_none() {
            return Err(TaskerError::ProcessNotFound(pid))
        }

        Ok(())
    }

    /// Associe un thread à un processus courant.
    pub fn add_thread(&mut self, pid: PId, tid: TId) -> TaskerResult<()> {
        self.processes
            .get_mut(&pid)
            .ok_or(TaskerError::ProcessNotFound(pid))?
            .add_thread(tid)
    }
}
