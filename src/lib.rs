#![no_std]
#![no_main]

pub mod vga_buffer;

use core::panic::PanicInfo;
use core::arch::asm;

// "no_mangle" garde le nom "_start" intact pour que l'assembleur le trouve
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Welcome to K Os.");

    hlt();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    hlt();
}

fn hlt() -> ! {
    loop {
        unsafe{
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
