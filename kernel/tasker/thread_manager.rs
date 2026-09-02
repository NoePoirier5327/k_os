//! Module de gestion des threads.
//! Il les stockes et gère leurs accès en lecture et écriture.

pub mod thread;

use alloc::collections::btree_map::BTreeMap;
use super::process_manager::process::PId;
use crate::tasker::{TaskerResult, TaskerError};
use thread::{TId, Thread};

/// Struture de gestion des threads de l'os.
pub struct ThreadManager {
    threads: BTreeMap<TId, Thread>,
}

impl ThreadManager {
    /// Instancie un nouveau gestionnaire de thread.
    pub fn new() -> Self {
        Self {
            threads: BTreeMap::new()
        }
    }

    /// Instancie un nouveau thread kernel et renvoie son identifiant.
    ///
    /// # Arguments
    /// * `parent_pid`: identifiant du processus parent auquel il est associé.
    /// * `entry`: point d'entré pour l'exécution du nouveau thread.
    /// * `kernel_stack_top`: adresse de haut de la pile kernel allouée au nouveau thread.
    ///
    /// # Return
    /// Identifiant du nouveau thread.
    pub fn create_kernel_thread(
        &mut self,
        parent_pid: PId,
        entry: u64,
        kernel_stack_top: u64
    ) -> TId {
        let thread = Thread::new_kernel(parent_pid, entry, kernel_stack_top);
        let tid = thread.get_tid();
        self.threads.insert(tid, thread);
        tid
    }

    /// Instancie un nouveau thread utilisateur et renvoie son identifiant.
    ///
    /// # Arguments
    /// * `parent_pid`: identifiant du processus parent auquel il est associé.
    /// * `entry`: point d'entré pour l'exécution du nouveau thread.
    /// * `user_stack_top`: haut de la pile d'exécution utilisateur allouée au nouveau thread.
    /// * `kernel_stack_top`: haut de la pile d'exécution kernel allouée au nouveau thread.
    ///
    /// # Return
    /// Identifiant du nouveau thread.
    pub fn create_user_thread(
        &mut self,
        parent_pid: PId,
        entry: u64,
        user_stack_top: u64,
        kernel_stack_top: u64
    ) -> TId {
        let thread = Thread::new_user(parent_pid, entry, user_stack_top, kernel_stack_top);
        let tid = thread.get_tid();
        self.threads.insert(tid, thread);
        tid
    }

    /// Renvoie, si trouver, un emprunt non mutable vers le thread associé à l'identifiant en
    /// paramètre.
    pub fn get(&self, tid: TId) -> TaskerResult<&Thread> {
        if let Some(thread) = self.threads.get(&tid) {
            return Ok(thread);
        }

        Err(TaskerError::ThreadNotFound(tid))
    }

    /// Renvoie, si trouver, un emprunt mutable vers le thread associé à l'identifiant en paramètre.
    pub fn get_mut(&mut self, tid: TId) -> TaskerResult<&mut Thread> {
        if let Some(thread) = self.threads.get_mut(&tid) {
            return Ok(thread);
        }

        Err(TaskerError::ThreadNotFound(tid))
    }

    /// Détruis le thread interne associé à l'identifiant en paramètre.
    /// Renvoie une erreur si le thread à supprimer n'existe pas.
    pub fn destroy(&mut self, tid: TId) -> TaskerResult<()> {
        if self.threads.remove(&tid).is_none() {
            return Err(TaskerError::ThreadNotFound(tid))
        }

        Ok(())
    }
}
