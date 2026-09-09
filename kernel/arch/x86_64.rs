//! Module de gestion de l'architecture x86_64.

pub mod gdt;
pub mod interrupts;
pub mod stack;
pub mod syscalls;
pub mod memory;

pub const USER_PAGES_START: u64 = 0x0000_0000_0000_0000u64;
pub const USER_PAGES_END: u64 = 0x0000_7FFF_FFFF_FFFFu64;

pub const KERNEL_PAGES_START: u64 = 0xFFFF_8000_0000_0000u64;
pub const KERNEL_PAGES_END: u64 = 0xFFFF_FFFF_FFFF_FFFFu64;
