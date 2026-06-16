//! Fichier principal du kernel, chargé par grub au démarrage dans start.asm.

// TODO Implémenter un guard page sous les piles d'appels pour gérer le stack overflow.

#![no_std]
#![no_main]

#![feature(abi_x86_interrupt)]

extern crate alloc;

pub mod user;
pub mod kernel;
pub mod vga_buffer;
pub mod interrupts;
pub mod gdt;
pub mod memory;
pub mod allocator;

use core::panic::PanicInfo;
use multiboot2::{BootInformation, BootInformationHeader};
use x86_64::VirtAddr;


extern "C" {
    static __kernel_start : u8;
    static __kernel_end : u8;
}


/// Fonction principal du noyau, elle est appelée par grub après son chargement.<br>
/// "no_mangle" garde le nom "_start" intact pour que l'assembleur le trouve.
///
/// # Argument
/// * `multiboot_info_ptr` : pointeur multiboot2 permettant la cartographie de la mémoire pour être utilisé par le noyau ensuite.
/// * `physical_memory_offset` : indice de décalage de pagination mémoire, envoyé depuis l'assembleur.
#[unsafe(no_mangle)]
pub extern "C" fn _start(multiboot_info_ptr : u64, physical_memory_offset : u64) -> ! {   
    crate::disp_info!("Kernel starts at 0x{:x}", core::ptr::addr_of!(__kernel_start) as u64);
    crate::disp_info!("Kernel ends at 0x{:x}", core::ptr::addr_of!(__kernel_end) as u64);

    // Vérification du format du pointeur multiboot.
    if !multiboot_info_ptr.is_multiple_of(8) {
        crate::disp_warning!("Unaligned multiboot2 pointer.");
    }

    if multiboot_info_ptr == 0 {
        panic!("The multiboot2 info pointer is NULL.");
    }

    crate::disp_info!("Multiboot2 info pointer = 0x{}", multiboot_info_ptr);
    crate::disp_info!("Physical memory offset = 0x{}", physical_memory_offset);

    // On initialise les composantes du kernel
    kernel::init();

    // Fabriquation de la carte de la mémoire à partir du pointeur multiboot_info
    let boot_info = unsafe { BootInformation::load(multiboot_info_ptr as *const BootInformationHeader).unwrap() };
    let memory_map_tag = unsafe {
        let tag = boot_info.memory_map_tag().expect("Memory map tag required.");
        &*(tag as *const multiboot2::MemoryMapTag)
    };

    let vritual_memory_offset = VirtAddr::new(physical_memory_offset);

    // Création des alloueurs mémoire.
    crate::disp_info!("Frame allocator initialization.");
    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(memory_map_tag) };
    let mut mapper = unsafe { memory::init(vritual_memory_offset) };

    // Allocation de la zone du tas.
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("Heap initialization failed.");

    // On affiche les informations d'aligement de la mémoire.
    crate::disp_info!("User stack will start at 0x{:x}.", user::USER_STACK_START);
    crate::disp_info!("User stack will end at 0x{:x}.", user::USER_STACK_START+user::USER_STACK_SIZE as u64-1);
    crate::disp_info!("User pages starts at 0x{:x}.", memory::USER_PAGES_START);
    crate::disp_info!("User pages ends at 0x{:x}.", memory::KERNEL_PAGES_START-1);
    crate::disp_info!("Kernel pages starts at 0x{:x}.", memory::KERNEL_PAGES_START);
    crate::disp_info!("Kernel pages ends at 0x{:x}.", allocator::HEAP_START-1);
    crate::disp_info!("Heap starts at 0x{:x}.", allocator::HEAP_START);
    crate::disp_info!("Head ends at 0x{:x}.", allocator::HEAP_START+allocator::HEAP_SIZE-1);

    // On passe en ring 3
    user::enter_user_space(&mut mapper, &mut frame_allocator);

    hlt();
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

    hlt();
}

/// Fonction d'arrêt du processeur en fonction du processeur.<br>
// TODO le faire fonctionner pour d'autres architectures que le x86_64.
fn hlt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
