//! Fichier principal du kernel, chargé par grub au démarrage dans start.asm.

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
pub mod syscall;

use core::panic::PanicInfo;
use kernel::Kernel;
use arch::{INTERRUPTION_CONTROLLER, CPU_CONTEXT, SYSCALL_INTERFACE};
use arch::hal::instructions::hlt_loop;
use arch::hal::interrupts::InterruptionController;
use arch::hal::cpu::CpuContext;
use arch::hal::syscalls::SyscallInterface;
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
    // On initialise les composantes mémoire globale avant le kernel.
    let phys_mem_offset = 0xFFFF_8000_0000_0000u64;

    // On initialise l'affichage.
    vga_buffer::init(phys_mem_offset);

    // On vérifie la validitée du pointeur multiboot2 en paramètre.
    if multiboot2_info_ptr == 0 {
        panic!("The multiboot2 information pointer is null.");
    }

    if !multiboot2_info_ptr.is_multiple_of(8) {
        crate::disp_warning!("Unaligned multiboot2 information pointer.");
    }

    crate::disp_info!("Initialization of the cpu execution context.");
    CPU_CONTEXT.init();

    crate::disp_info!("Initialization of the interruption controller.");
    { INTERRUPTION_CONTROLLER.lock().init(); }

    // On initialise le kernel.
    crate::disp_info!("Initialization of the kernel module.");
    Kernel::init(phys_mem_offset, multiboot2_info_ptr);

    crate::disp_info!("Initialization of the kernel heap.");
    Kernel::with_memory(|frame_allocator, mapper| {
        memory::heap::init_heap(mapper, frame_allocator)
            .expect("Failed to initialize kernel's heap ");
    });

    crate::disp_info!("Initialization of the tasker.");
    Tasker::init();

    crate::disp_info!("Initialization of the syscall support.");
    SYSCALL_INTERFACE.init();

    crate::disp_info!("Enabling cpu's interruptions.");
    x86_64::instructions::interrupts::enable();

    Tasker::on_instance(|tasker| {
        // On créer un processus kernel à deux threads.
        let kernel_pid = tasker.create_kernel_process("Test", test1 as *const () as usize as u64)
            .expect("An error occured during a kernel process creation ");
        tasker.create_kernel_thread(kernel_pid, test2 as *const () as usize as u64)
            .expect("An error occured during a kernel thread creation ");

        // Qu'on supperpose à un processus utilisateur monothread.
        /*
        static ELF_BYTES: &AlignedElfBinary<[u8]> = &AlignedElfBinary(*include_bytes!("../user/hello_world/hello"));
        let _ = tasker.create_user_process("Hello", &ELF_BYTES.0)
            .expect("An error occured during a user process creation ");
        */
    });

    arch::hal::instructions::hlt_loop();
}

/// Interface de gestion de la panique.
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
