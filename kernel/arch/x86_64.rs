//! Module de gestion de l'architecture x86_64.

pub mod gdt;
pub mod interrupts;
pub mod syscalls;
pub mod memory;

use crate::arch::hal::memory::MemoryLayout;

pub static X86_64MEMORY_LAYOUT: X86_64MemoryLayout = X86_64MemoryLayout {
    user_start:  0x0000_0000_0000_1000u64,
    user_end: 0x0000_7FFF_FFFF_FFFFu64,
    kernel_start: 0xFFFF_8000_0000_0000u64,
    kernel_end: 0xFFFF_FFFF_FFFF_FFFFu64,
    stack_top: 0x0000_7FFF_FFFF_0000u64,
    stack_guard_size: 4 * 1024,
};

/// Représente l'organisation de la mémoire de l'architecture x86_64.
pub struct X86_64MemoryLayout {
    user_start: u64,
    user_end: u64,
    kernel_start: u64,
    kernel_end: u64,
    stack_top: u64,
    stack_guard_size: usize
}

impl MemoryLayout for X86_64MemoryLayout {
    fn get_user_start(&self) -> u64 {
        self.user_start
    }

    fn get_user_end(&self) -> u64 {
        self.user_end
    }

    fn get_kernel_start(&self) -> u64 {
        self.kernel_start
    }

    fn get_kernel_end(&self) -> u64 {
        self.kernel_end
    }

    fn get_stack_top(&self) -> u64 {
        self.stack_top
    }

    fn get_stack_guard_size(&self) -> usize {
        self.stack_guard_size
    }
}
