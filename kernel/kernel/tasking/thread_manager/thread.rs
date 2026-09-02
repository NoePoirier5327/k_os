//! Module de gestion de Thread.
//! Un thread doit forcément être associé à un processus parent.

use core::sync::atomic::{AtomicUsize, Ordering};
use super::super::process_manager::process::PId;

/// Identifiant d'un thread.
/// Sert à se référer à un thread dans un processus.
pub type TId = usize;

/// Renvoie l'identifiant du prochain thread à instancier.
static NEXT_TID: AtomicUsize = AtomicUsize::new(1usize);

/// Représente un thread attâché à un processus parent.
pub struct Thread {
    tid: TId,
    parent_pid: PId,
    context: CpuContext,
    state: ThreadState,
    kernel_stack_top: u64,
    user_stack_top: Option<u64>
}

impl Thread {
    /// Fonction de création d'un nouveau thread kernel.
    ///
    /// # Arguments
    /// * `parent_pid`: Identifiant du processus parent auquel il est rattaché.
    /// * `entry`: Adresse d'entrée du nouveau processus.
    /// * `kernel_stack_top`: Adresse du haut de la pile kernel associé au thread courant.
    ///
    /// # Return
    /// Nouveau thread kernel associé au point d'entré en paramètre.
    pub fn new_kernel(parent_pid: PId, entry: u64, kernel_stack_top: u64) -> Self {
        Self {
            tid: NEXT_TID.fetch_add(1usize, Ordering::Relaxed),
            parent_pid,
            state: ThreadState::Ready,
            context: CpuContext {
                rip: entry,
                rsp: kernel_stack_top,
                rflags: 0x202,
                registers: [0u64; 16]
            },
            kernel_stack_top,
            user_stack_top: None
        }
    }

    /// Fonction de création d'un nouveau thread utilisateur.
    ///
    /// # Arguments
    /// * `parent_pid`: Identifiant du processus parent auquel le thread sera associé.
    /// * `entry`: Point d'entré de l'exécution du nouveau thread.
    /// * `user_stack_top`: Adresse du haut de la pile utilisateur allouée au thread.
    /// * `kernel_stack_top`: Adresse du haut de la pile kernel allouée au thread.
    ///
    /// # Return
    /// Nouveau thread utilisateur associé au point d'entré en paramètre.
    pub fn new_user(parent_pid: PId, entry: u64, user_stack_top: u64, kernel_stack_top: u64) -> Self {
        Self {
            tid: NEXT_TID.fetch_add(1usize, Ordering::Relaxed),
            parent_pid,
            state: ThreadState::Ready,
            context: CpuContext { 
                rip: entry,
                rsp: user_stack_top,
                rflags: 0x202,
                registers: [0u64; 16]
            },
            kernel_stack_top,
            user_stack_top: Some(user_stack_top)
        }
    }

    /// Renvoie l'identifiant du thread courant.
    pub fn get_tid(&self) -> TId {
        self.tid
    }

    /// Renvoie l'identifiant du processus parent.
    pub fn get_parent_pid(&self) -> PId {
        self.parent_pid
    }

    /// Renvoie l'état du thread courant.
    pub fn get_state(&self) -> ThreadState {
        self.state
    }
}

/// Représente l'état d'execution d'un thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Busy,
    Blocked,
    Dead
}

/// Représente le context d'exécution CPU d'un thread.
#[derive(Clone, Copy)]
pub struct CpuContext {
    pub rip: u64,
    pub rsp: u64,
    pub rflags: u64,
    pub registers: [u64; 16]
}

impl CpuContext {
    pub fn empty() -> Self {
        Self {
            rip: 0u64,
            rsp: 0u64,
            rflags: 0u64,
            registers: [0u64; 16]
        }
    }
}
