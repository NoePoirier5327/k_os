//! Module de gestion de Thread.
//! Un thread doit forcément être associé à un processus parent.

use core::sync::atomic::{AtomicUsize, Ordering};
use super::super::process_manager::process::PId;
use crate::arch::hal::memory::MapperTrait;
use crate::arch::hal::interrupts::{new_kernel_interruption_stack_frame, new_user_interruption_stack_frame};
use crate::memory::stack::{KernelStack16Kib, UserStack16Kib};
use crate::memory::types::VirtAddr;
use crate::tasker::{TaskerResult, TaskerError};

/// Identifiant d'un thread.
/// Sert à se référer à un thread dans un processus.
pub type TId = usize;

/// Renvoie l'identifiant du prochain thread à instancier.
static NEXT_TID: AtomicUsize = AtomicUsize::new(1usize);

/// Représente un thread attâché à un processus parent.
pub struct Thread {
    tid: TId,
    parent_pid: PId,
    pub rsp: u64,
    pub state: ThreadState,
    kernel_stack: KernelStack16Kib,
    user_stack: Option<UserStack16Kib>,
    user_stack_top: Option<VirtAddr>
}

impl Thread {
    /// Fonction de création d'un nouveau thread kernel.
    ///
    /// # Arguments
    /// * `parent_pid`: Identifiant du processus parent auquel il est rattaché.
    /// * `entry_point`: Adresse d'entrée du nouveau processus.
    /// * `kernel_stack`: Pile kernel allouée pour le nouveau thread.
    ///
    /// # Return
    /// Nouveau thread kernel associé au point d'entré en paramètre.
    pub fn new_kernel(parent_pid: PId, entry_point: u64, kernel_stack: KernelStack16Kib) -> Self {
        Self {
            tid: NEXT_TID.fetch_add(1usize, Ordering::Relaxed),
            parent_pid,
            state: ThreadState::Ready,
            rsp: unsafe { 
                new_kernel_interruption_stack_frame(
                    kernel_stack.get_top_vaddr(),
                    entry_point
                ).as_u64()
            },
            kernel_stack,
            user_stack: None,
            user_stack_top: None
        }
    }

    /// Fonction de création d'un nouveau thread utilisateur.
    ///
    /// # Arguments
    /// * `parent_pid`: Identifiant du processus parent auquel le thread sera associé.
    /// * `entry_point`: Point d'entré de l'exécution du nouveau thread.
    /// * `user_stack_top`: Haut de la pile utilisateur allouée au thread.
    /// * `kernel_stack`: Pile kernel allouée au thread.
    ///
    /// # Return
    /// Nouveau thread utilisateur associé au point d'entré en paramètre.
    pub fn new_user(parent_pid: PId, entry_point: u64, user_stack_top: VirtAddr, kernel_stack: KernelStack16Kib, user_stack: UserStack16Kib) -> Self {
        Self {
            tid: NEXT_TID.fetch_add(1usize, Ordering::Relaxed),
            parent_pid,
            state: ThreadState::Ready,
            rsp: unsafe {
                new_user_interruption_stack_frame(
                    user_stack_top,
                    kernel_stack.get_top_vaddr(),
                    entry_point
                ).as_u64()
            },
            kernel_stack,
            user_stack: Some(user_stack),
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

    /// Met l'état du thread courant à mort
    pub fn kill(&mut self) {
        self.state = ThreadState::Dead;
    }

    /// Renvoie l'adresse du haut de la pile kernel associé au thread courant.
    pub fn get_kernel_stack_top(&self) -> u64 {
        self.kernel_stack.get_top_vaddr().as_u64()
    }

    /// Renvoie l'adresse virtuelle du haut de la pile kernel interne.
    pub fn get_kernel_top_vaddr(&self) -> VirtAddr {
        self.kernel_stack.get_top_vaddr()
    }

    /// Désalloue la pile kernel interne au thread.
    pub fn deallocate_kernel_stack(&mut self) {
        unsafe { self.kernel_stack.deallocate() };
    }

    /// désalloue la pile utilisateur interne au thread.
    pub fn deallocate_user_stack(
        &mut self,
        user_mapper: &mut dyn MapperTrait
    ) -> Result<(), TaskerError> {
        if let Some(user_stack) = &mut self.user_stack {
            unsafe { user_stack.deallocate(user_mapper); }
            return Ok(())
        }

        Err(TaskerError::WrongProcessKind)
    }

    /// Renvoie l'adresse virtuelle du haut de la pile utilisateur.
    /// Renvoie une erreur si user_stack non défini
    pub fn get_user_stack_top_vaddr(&self) -> TaskerResult<VirtAddr> {
        if let Some(stack) = self.user_stack_top {
            return Ok(stack)
        }
        Err(TaskerError::WrongProcessKind)
    }
}

/// Représente l'état d'execution d'un thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Running,
    Blocked,
    Dead
}
