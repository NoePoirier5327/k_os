//! Composante du module de gestion de la mémoire.
//! Gère spécifiquement les aspects de la mémoire lié au processeur.

/// Représente le contexte d'exécution du cpu à un moment donné.
/// Utile pour le multithreading.
#[repr(C)]
pub struct CpuContext {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64
}

/// Représente une frame d'interruption.
/// Utile pour le multithreading.
#[repr(C)]
pub struct InterruptFrame {
    // Ici à chaque interruption.
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,

    // Uniquement si changement de privilège.
    pub rsp: u64,
    pub ss: u64
}
