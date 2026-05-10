#![no_std]
#![no_main]

#![feature(abi_x86_interrupt)]

pub mod vga_buffer;
pub mod interrupts;

use core::panic::PanicInfo;
use core::arch::asm;

// "no_mangle" garde le nom "_start" intact pour que l'assembleur le trouve
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Welcome to k_os.");

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
