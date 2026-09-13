//! Module de gestion de l'architecture x86_64.

pub mod gdt;
pub mod interrupts;
pub mod syscalls;
pub mod memory;

use crate::arch::hal::memory::MemoryLayout;

pub static X86_64MEMORY_LAYOUT: X86_64MemoryLayout = X86_64MemoryLayout {
    user_pages_start:           0x0000_0000_0000_1000u64,
        user_stack_region_start:    0x0000_7FFF_FFFF_0000u64,
        user_stack_region_end:      0x0000_7FFF_FFFF_FFFEu64,
    user_pages_end:             0x0000_7FFF_FFFF_FFFFu64,


    kernel_pages_start:         0xFFFF_8000_0000_0000u64,
        kernel_heap_start:          0xFFFF_9000_0000_0000u64,
        kernel_heap_end:            0xFFFF_9000_004F_FFFFu64,

        kernel_stack_region_start:  0xFFFF_FF80_0000_0000u64,
        kernel_stack_region_end:    0xFFFF_FFFF_FFFF_FFFEu64,
    kernel_pages_end:           0xFFFF_FFFF_FFFF_FFFFu64,


    stack_guard_size: 4 * 1024,
};

/// Représente l'organisation de la mémoire de l'architecture x86_64.
pub struct X86_64MemoryLayout {
    user_pages_start: u64,
    user_stack_region_start: u64,
    user_stack_region_end: u64,
    user_pages_end: u64,
    kernel_pages_start: u64,
    kernel_heap_start: u64,
    kernel_heap_end: u64,
    kernel_stack_region_start: u64,
    kernel_stack_region_end: u64,
    kernel_pages_end: u64,
    stack_guard_size: usize
}

impl MemoryLayout for X86_64MemoryLayout {
    fn get_user_pages_start(&self) -> u64 {
        self.user_pages_start
    }

    fn get_user_pages_end(&self) -> u64 {
        self.user_pages_end
    }

    fn get_kernel_pages_start(&self) -> u64 {
        self.kernel_pages_start
    }

    fn get_kernel_pages_end(&self) -> u64 {
        self.kernel_pages_end
    }

    fn get_kernel_heap_start(&self) -> u64 {
        self.kernel_heap_start
    }

    fn get_kernel_heap_end(&self) -> u64 {
        self.kernel_heap_end
    }

    fn get_kernel_stack_region_start(&self) -> u64 {
        self.kernel_stack_region_start
    }

    fn get_kernel_stack_region_end(&self) -> u64 {
        self.kernel_stack_region_end
    }

    fn get_user_stack_region_start(&self) -> u64 {
        self.user_stack_region_start
    }

    fn get_user_stack_regions_end(&self) -> u64 {
        self.user_stack_region_end
    }

    fn get_stack_guard_size(&self) -> usize {
        self.stack_guard_size
    }
}
