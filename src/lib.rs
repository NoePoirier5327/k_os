#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::arch::asm;

// "no_mangle" garde le nom "_start" intact pour que l'assembleur le trouve
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let vga_buffer = 0xb8000 as *mut u8;
    unsafe {
        *vga_buffer = b'O';
        *vga_buffer.offset(1) = 0x07;
    }

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
