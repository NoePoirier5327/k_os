//! Fichier principal du kernel, chargé par grub au démarrage dans start.asm.

// TODO Implémenter un guard page sous les piles d'appels pour gérer le stack overflow.

#![no_std]
#![no_main]

#![feature(abi_x86_interrupt)]

pub mod vga_buffer;
pub mod interrupts;
pub mod gdt;

use core::panic::PanicInfo;
use core::arch::asm;

// "no_mangle" garde le nom "_start" intact pour que l'assembleur le trouve
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    init();
    println!("Welcome to k_os.");

    x86_64::instructions::interrupts::int3();

    unsafe {
        *(0xdeadbeef as *mut u64) = 42;
    };

    hlt();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt();
}

/// Fonction d'arrêt du processeur en fonction du processeur.<br>
/// TODO le faire fonctionner pour d'autres architectures que le x86_64.
fn hlt() -> ! {
    loop {
        unsafe{
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

/// Fonction d'initialisation des composantes de sécurité processeur comme la IDT.
fn init() {
    gdt::init();
    interrupts::init_idt();
}
