//! Module de gestion des architectures cibles pour le système d'exploitation.
// NOTE Ne supporte que le x86_64 pour le moment.

mod x86_64;

/// Module d'abstraction global, c'est à lui que seront fait les appelles.
pub mod hal;

// Définition des interface génériques d'appelle aux interruptions
#[cfg(target_arch = "x86_64")]
pub use x86_64::interrupts::PIC_CONTROLLER as INTERRUPTION_CONTROLLER;

#[cfg(target_arch = "x86_64")]
pub use x86_64::interrupts::without_interrupts;

// Aux contextes d'exécutions
#[cfg(target_arch = "x86_64")]
pub use x86_64::gdt::X86_64CPU_CONTEXT_INTERFACE as CPU_CONTEXT;

// Aux interfaces d'appels systèmes
#[cfg(target_arch = "x86_64")]
pub use x86_64::syscalls::X86_64SYSCALL_INTERFACE as SYSCALL_INTERFACE;

// On exporte les organisations de la mémoire en fonction de l'architecture cible.
#[cfg(target_arch = "x86_64")]
pub use x86_64::X86_64MemoryLayout as MEMORY_LAYOUT;
