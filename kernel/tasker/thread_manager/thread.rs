//! Module de gestion de Thread.
//! Un thread doit forcément être associé à un processus parent.

use core::sync::atomic::{AtomicUsize, Ordering};
use super::super::process_manager::process::PId;
use crate::memory::cpu::CpuContext;
use crate::arch::x86_64::gdt;

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
        let mut rsp = kernel_stack_top;

        unsafe {
            // Alignement et écriture de la frame initiale sur la pile
            let context_ptr = (rsp - core::mem::size_of::<CpuContext>() as u64) as *mut CpuContext;
        
            *context_ptr = CpuContext {
                // Frame d'interruption matérielle
                ss: 0x10,           // Segment de données Kernel (GDT)
                rsp: kernel_stack_top,
                rflags: 0x202,      // Interrupts enabled (IF = 1)
                cs: 0x08,           // Segment de code Kernel (GDT)
                rip: entry,
            
                // Registres initiaux à zéro
                rax: 0, rbx: 0, rcx: 0, rdx: 0,
                rsi: 0, rdi: 0, rbp: 0, r8: 0,
                r9: 0, r10: 0, r11: 0, r12: 0,
                r13: 0, r14: 0, r15: 0,
            };

            rsp = context_ptr as u64;
        }

        Self {
            tid: NEXT_TID.fetch_add(1usize, Ordering::Relaxed),
            parent_pid,
            state: ThreadState::Ready,
            rsp,
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
        let mut rsp = kernel_stack_top;

        unsafe {
            // Écriture du CpuContext en haut de la pile KERNEL du thread
            let context_ptr = (rsp - core::mem::size_of::<CpuContext>() as u64) as *mut CpuContext;

            *context_ptr = CpuContext {
                // Frame d'interruption pour le saut en Ring 3 via iretq
                ss: gdt::get_selectors().get_user_data_selector().0 as u64,
                rsp: user_stack_top, // Pile utilisateur appliquée au retour de l'interruption
                rflags: 0x202,       // Interruptions activées (IF = 1)
                cs: gdt::get_selectors().get_user_code_selector().0 as u64,
                rip: entry,          // Point d'entrée en espace utilisateur

                // Registres généraux initialisés à zéro
                rax: 0, rbx: 0, rcx: 0, rdx: 0,
                rsi: 0, rdi: 0, rbp: 0, r8: 0,
                r9: 0, r10: 0, r11: 0, r12: 0,
                r13: 0, r14: 0, r15: 0,
            };

            rsp = context_ptr as u64;
        }

        Self {
            tid: NEXT_TID.fetch_add(1usize, Ordering::Relaxed),
            parent_pid,
            state: ThreadState::Ready,
            rsp,
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

    /// Met l'état du thread courant à mort
    pub fn kill(&mut self) {
        self.state = ThreadState::Dead;
    }

    /// Renvoie l'adresse du haut de la pile kernel associé au thread courant.
    pub fn get_kernel_stack_top(&self) -> u64 {
        self.kernel_stack_top
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
