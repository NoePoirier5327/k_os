//! Implémentation d'un ordonnanceur de threads mémoire correspondant 
//! à l'algorithme Round Robin.

pub mod thread;

use thread::{Thread, ThreadId, ThreadState, swap_context};
use alloc::collections::VecDeque;
use crate::kernel::Kernel;


/// Type représentant un ordonnanceur de thread. <br>
/// Ne doit être accéder que depuis le kernel.
pub struct Scheduler {
    threads : VecDeque<Thread>,
    next_thread : ThreadId, // Identifiant du prochain thread à créer
    garbage : Option<Thread>
}

impl Scheduler {
    /// Création d'un ordonnanceur évaluable à la compilation.
    pub const fn new() -> Self {
        Self {
            threads : VecDeque::new(),
            next_thread : 0,
            garbage : None
        }
    }

    /// Fonction de création d'un thread dans l'ordonnanceur courant.
    ///
    /// # Argument
    /// * `entry_point` : processus à atacher au nouveau thread.
    pub fn spawn(&mut self, entry_point : fn()) {
        let entry_point_adr = entry_point as *const () as usize;
        let stack_size = 16_384;

        let new_thread = Thread::new(self.next_thread, entry_point_adr, stack_size);

        self.threads.push_back(new_thread);
        self.next_thread += 1;
    }

    /// Accesseur du thread courant.<br>
    /// Renvoie `None` si l'ordonnanceur ne possède aucun thread à renvoyer.
    pub fn get_current_thread(&self) -> Option<&Thread> {
        self.threads.front()
    }

    /// Accesseur muable sur le thread courant.<br>
    /// Renvoie `None` si l'ordonnanceur ne possède aucun thread à renvoyer.
    pub fn get_current_thread_mut(&mut self) -> Option<&mut Thread> {
        self.threads.front_mut()
    }

    /// Méthode d'acquisition des adresses de changement de contexte processeur.
    pub fn get_context(&mut self) -> Option<(*mut u64, u64)> {
        // On libert l'ancien thread mort.
        self.garbage = None;

        // S'il n'y a qu'un seul processus ou moins dans l'ordonnanceur,
        // aucun besoin de la méthode.
        if self.threads.len() <= 1 {
            return None;
        }

        let is_dead = self.threads.front().unwrap().get_state() == ThreadState::Dead;

        if is_dead {
            // On récupère le thread mort pour laisser rust le déallouer.
            self.garbage = self.threads.pop_front();

            // On occupe le nouveau thread courant
            let next_thread = self.threads.front_mut().unwrap();
            next_thread.busy();
            let next_rsp = next_thread.get_stack_pointer();

            // On renvoie le nouveau contexte
            let mut temp_rsp = 0u64;
            Some((&mut temp_rsp as *mut u64, next_rsp))
        } else {
            self.threads.front_mut().unwrap().ready();
            self.threads.rotate_left(1);
            self.threads.front_mut().unwrap().busy();

            // On récupère les pointeurs de pile servant au changement de contexte.
            let new_rsp = self.threads.front().unwrap().get_stack_pointer();
            let old_rsp = self.threads.back_mut().unwrap().get_stack_pointer_mut() as *mut u64;

            // On renvoie le nouveau contexte
            Some((old_rsp, new_rsp))
        }
    }
}

/// Fonction de changement de contexte de l'ordonnanceur mémoire.
pub fn schedule() {
    let context_to_swap = Kernel::with_scheduler(|scheduler| {
        scheduler.get_context()
    });

    // On effectue le changement de contexte ici
    if let Some((old_rsp, new_rsp)) = context_to_swap {
        unsafe {
            swap_context(old_rsp, new_rsp);
        }
    }
}
