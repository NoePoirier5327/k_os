//! Fichier principal du kernel, chargé par grub au démarrage dans start.asm.

// TODO Implémenter un guard page sous les piles d'appels pour gérer le stack overflow.

#![no_std]
#![no_main]

#![feature(abi_x86_interrupt)]

pub mod vga_buffer;
pub mod interrupts;
pub mod gdt;
pub mod memory;

use core::panic::PanicInfo;
use multiboot2::{BootInformation, BootInformationHeader};
use x86_64::VirtAddr;
use x86_64::structures::paging::PageTable;

/// Fonction principal du noyau, elle est appelée par grub après son chargement.<br>
/// "no_mangle" garde le nom "_start" intact pour que l'assembleur le trouve.
///
/// # Argument
/// * `multiboot_info_ptr` : pointeur multiboot2 permettant la cartographie de la mémoire pour être utilisé par le noyau ensuite.
/// * `physical_memory_offset` : indice de décalage de pagination mémoire, envoyé depuis l'assembleur.
#[unsafe(no_mangle)]
pub extern "C" fn _start(multiboot_info_ptr : u64, physical_memory_offset : u64) -> ! {
    // Vérification du format du pointeur multiboot.
    if !multiboot_info_ptr.is_multiple_of(8) {
        println!("WARNING: Unaligned multiboot pointer.");
    }

    if multiboot_info_ptr == 0 {
        println!("ERROR: The multiboot2 info pointer is NULL.");
    }

    println!("INFO: Multiboot2 info pointer = {}", multiboot_info_ptr);
    println!("INFO: Physical memory offset = {}", physical_memory_offset);

    // Fabriquation de la carte de la mémoire à partir du pointeur multiboot_info
    let boot_info = unsafe { BootInformation::load(multiboot_info_ptr as *const BootInformationHeader).unwrap() };
    let memory_map_tag = unsafe {
        let tag = boot_info.memory_map_tag().expect("Memory map tag required");
        &*(tag as *const multiboot2::MemoryMapTag)
    };

    let offset = VirtAddr::new(physical_memory_offset);

    // Création des alloueurs mémoire.
    println!("Frame allocator initialization.");
    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(memory_map_tag) };
    let mut mapper = unsafe { memory::init(offset) };

    // Initialisation des composantes du noyau.
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

/// Fonction d'initialisation des composantes du noyau comme la table d'interrutions et les ports x86_64
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
