//! Composante du module de gestion de la mémoire.
//! Gère spécifiquement les aspects de la mémoire lié au processeur.

use crate::arch::x86_64::gdt;

/// Représente le contexte d'exécution du cpu à un moment donné.
/// Utile pour le multithreading.
#[repr(C)]
pub struct CpuContext {
    // Registres généraux empilés manuellement (dans l'ordre inverse des PUSH)
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,

    // Empilés automatiquement par le CPU lors de l'interruption
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl CpuContext {
    /// Prépare le contexte initial pour la pile noyau d'un thread noyau.
    ///
    /// # Arguments
    /// * `kernel_stack_top`: haut de la pile kernel associée au contexte cible.
    /// * `entry_point`: adresse de la fonction à exécuter dans le contexte cible.
    ///
    /// # Return
    /// Pointeur vers la pile noyau préparée pour le nouveau thread.
    pub fn new_kernel(
        kernel_stack_top: u64,
        entry_point: u64,
    ) -> u64 {
        let selectors = gdt::get_selectors();
        let kernel_cs = selectors.get_kernel_code_selector().0 as u64;
        let kernel_ss = selectors.get_kernel_data_selector().0 as u64;

        let context_size = core::mem::size_of::<CpuContext>() as u64;
        let stack_ptr = kernel_stack_top - context_size;

        unsafe {
            // Alignement et écriture de la frame initiale sur la pile    
            let context_ptr = &mut *(stack_ptr as *mut CpuContext);
            *context_ptr = CpuContext {
                // Frame d'interruption matérielle
                ss: kernel_ss,
                rsp: kernel_stack_top,
                rflags: 0x202,      // Interrupts enabled (IF = 1)
                cs: kernel_cs,
                rip: entry_point,
            
                // Registres initiaux à zéro
                rax: 0, rbx: 0, rcx: 0, rdx: 0,
                rsi: 0, rdi: 0, rbp: 0, r8: 0,
                r9: 0, r10: 0, r11: 0, r12: 0,
                r13: 0, r14: 0, r15: 0,
            };
        }

        stack_ptr
    }

    /// Prépare le contexte initial sur la pile noyau pour basculer en Ring 3 via iretq.
    /// 
    /// # Arguments
    /// * `kernel_stack_top`: haut de la pile kernel associée au contexte cible.
    /// * `user_entry_point`: adresse de saut vers le ring3
    /// * `user_stack_top`: haut de la pile utilisateur associée au contexte cible.
    ///
    /// # Return
    /// Pointeur vers la pile kernel préparée pour le ring3.
    pub fn new_user(
        kernel_stack_top: u64,
        user_entry_point: u64,
        user_stack_top: u64,
    ) -> u64 {
        let selectors = gdt::get_selectors();

        // Récupération des sélecteurs avec RPL 3 (bits 0 et 1 mis à 1)
        let user_cs = selectors.get_user_code_selector().0 as u64;
        let user_ss = selectors.get_user_data_selector().0 as u64;

        // Calcul de l'emplacement de la structure en haut de la pile noyau
        let context_size = core::mem::size_of::<CpuContext>() as u64;
        let stack_ptr = kernel_stack_top - context_size;

        unsafe {
            let context_ptr = &mut *(stack_ptr as *mut CpuContext);

            *context_ptr = CpuContext {
                // Registres généraux initialisés à 0
                r15: 0, r14: 0, r13: 0, r12: 0,
                r11: 0, r10: 0, r9:  0, r8:  0,
                rbp: 0, rdi: 0, rsi: 0, rdx: 0,
                rcx: 0, rbx: 0, rax: 0,

                // Frame consommée par iretq
                rip: user_entry_point,
                cs: user_cs,
                rflags: 0x202, // Bit 1 = 1 (obligatoire), Bit 9 (IF) = 1 pour garder les interruptions actives
                rsp: user_stack_top,
                ss: user_ss,
            };
        }

        stack_ptr
    }
}
