//! Fichier principal du kernel, chargé par grub au démarrage dans start.asm.

// TODO Implémenter un guard page sous les piles d'appels pour gérer le stack overflow.

#![no_std]
#![no_main]

#![feature(abi_x86_interrupt)]

pub mod vga_buffer;
pub mod interrupts;
pub mod gdt;

use core::panic::PanicInfo;

// "no_mangle" garde le nom "_start" intact pour que l'assembleur le trouve
#[unsafe(no_mangle)]
pub extern "C" fn _start(multiboot_info_ptr : usize) -> ! {
    init();
    println!("Welcome to k_os.");

    hlt();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n{}", info);
    hlt();
}

/// Fonction d'arrêt du processeur en fonction du processeur.<br>
// TODO le faire fonctionner pour d'autres architectures que le x86_64.
fn hlt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Fonction d'initialisation des composantes de sécurité processeur comme la IDT.
fn init() {
    gdt::init();

    print!("IDT initialization ");
    interrupts::init_idt();
    print!("(OK)\n");

    print!("PICS initialization ");
    unsafe { interrupts::PICS.lock().initialize() };
    print!("(OK)\n");

    print!("Enabling CPU interruption ");
    x86_64::instructions::interrupts::enable();
    print!("(OK)\n");
}
