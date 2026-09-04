//! Fichier principal du kernel, chargé par grub au démarrage dans start.asm.

// TODO Implémenter un guard page sous les piles d'appels pour gérer le stack overflow.

#![no_std]
#![no_main]

#![feature(abi_x86_interrupt)]

extern crate alloc;

pub mod kernel;
pub mod tasker;
mod message;
pub mod vga_buffer;
pub mod memory;
pub mod arch;

use core::panic::PanicInfo;
use kernel::Kernel;
use tasker::Tasker;
use arch::x86_64::stack::Stack16Kib;

fn test_kernel() {
    loop {
        crate::disp_debug!("This is a test at kernel level.");
    }
}

static TEST_STACK: Stack16Kib = Stack16Kib::empty();

/// Fonction principal du noyau, elle est appelée par grub après son chargement.<br>
/// "no_mangle" garde le nom "_start" intact pour que l'assembleur le trouve.
///
/// # Argument
/// * `multiboot_info_ptr` : pointeur multiboot2 permettant la cartographie de la mémoire pour être utilisé par le noyau ensuite.
#[unsafe(no_mangle)]
pub extern "C" fn kernel_start(multiboot2_info_ptr : u64) -> ! {   
    Kernel::init(multiboot2_info_ptr);

    let (_pid, _tid) = Tasker::on_instance(|tasker| {
        let pid = tasker.create_kernel_process("test kernel");
        let tid = tasker.create_kernel_thread(pid, test_kernel as *const () as usize as u64, TEST_STACK.get_top());
        (pid, tid)
    });

    hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use vga_buffer::{set_writer_color, set_default_writer_color, Color};

    set_default_writer_color();
    print!("[");
    set_writer_color(Color::Red, Color::Black);
    print!("PANIC!");
    set_default_writer_color();
    println!("]\n{}", info);

    hlt_loop();
}

/// Fonction d'arrêt du processeur en fonction du processeur.<br>
// TODO le faire fonctionner pour d'autres architectures que le x86_64.
fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
