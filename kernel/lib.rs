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

fn test1() {
    loop {
        crate::disp_debug!("This is displayed by a kernel process.");
    }
}

fn test2() {
    loop {
        crate::disp_debug!("This is displayed by the same process but not the same thread.");
    }
}

/// Fonction principal du noyau, elle est appelée par grub après son chargement.<br>
/// "no_mangle" garde le nom "_start" intact pour que l'assembleur le trouve.
///
/// # Argument
/// * `multiboot_info_ptr` : pointeur multiboot2 permettant la cartographie de la mémoire pour être utilisé par le noyau ensuite.
#[unsafe(no_mangle)]
pub extern "C" fn kernel_start(multiboot2_info_ptr : u64) -> ! {   
    Kernel::init(multiboot2_info_ptr);

    Tasker::on_instance(|tasker| {
        let pid = tasker.create_kernel_process("Test", test1 as *const () as usize as u64)
            .expect("An error occured during a kernel process creation ");
        tasker.create_kernel_thread(pid, test2 as *const () as usize as u64)
            .expect("An error occured during a kernel thread creation ");
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
