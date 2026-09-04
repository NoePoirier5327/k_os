//! Ordonnanceur de thread du module tasking.

use alloc::collections::vec_deque::VecDeque;
use crate::tasker::{TaskerError, TaskerResult};
use crate::tasker::thread_manager::thread::TId;

pub struct Scheduler {
    ready_queue: VecDeque<TId>,
    current: Option<TId>
}

impl Scheduler {
    /// Instancie un nouvel ordonnanceur de thread.
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            current: None
        }
    }

    /// Ajoute un nouveau thread à la file d'exécution.
    /// Renvoie une erreur s'il existe déjà dans l'ordonnanceur.
    pub fn add_thread(&mut self, tid: TId) -> TaskerResult<()> {
        if self.ready_queue.contains(&tid) {
            return Err(TaskerError::AlreadyExists)
        }

        self.ready_queue.push_back(tid);
        Ok(())
    }

    /// Supprime un thread de l'ordonnanceur.
    /// Renvoie une erreur s'il est introuvable.
    pub fn remove_thread(&mut self, tid: TId) -> TaskerResult<()> {
        if let Some(pos) = self.ready_queue.iter().position(|&t| t == tid) {
            self.ready_queue.remove(pos);
            return Ok(())
        }
        Err(TaskerError::ThreadNotFound(tid))
    }

    /// Réorganise et renvoie le prochain thread.
    pub fn pick_next(&mut self) -> Option<TId> {
        let tid = self.ready_queue.pop_front()?;
        self.current = Some(tid);
        Some(tid)
    }

    /// Renvoie l'identifiant du thread courant.
    pub fn get_current(&self) -> Option<TId> {
        self.current
    }
}
