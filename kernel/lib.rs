//! Fichier principal du kernel, chargé par grub au démarrage dans start.asm.

// TODO Implémenter un guard page sous les piles d'appels pour gérer le stack overflow.

#![no_std]
#![no_main]

#![feature(abi_x86_interrupt)]

extern crate alloc;

pub mod user_mode;
pub mod interrupts;
pub mod gdt;
pub mod memory;
pub mod allocator;
pub mod scheduler;
pub mod message;
pub mod vga_buffer;
pub mod syscalls;
pub mod elf;

use core::panic::PanicInfo;
use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};
use multiboot2::{BootInformation, BootInformationHeader};
use x86_64::VirtAddr;
use gdt::get_selectors;


extern "C" {
    static __kernel_start : u8;
    static __kernel_end : u8;
}

/// Offset de la mémoire pysique
pub const PHYSICAL_MEMORY_OFFSET: u64 = 0xFFFF_FFFF_8000_0000;

/// Offset de la mémoire virtuelle
pub const VIRTUAL_MEMORY_OFFSET: VirtAddr = VirtAddr::new(PHYSICAL_MEMORY_OFFSET);


/// Fonction principal du noyau, elle est appelée par grub après son chargement.<br>
/// "no_mangle" garde le nom "_start" intact pour que l'assembleur le trouve.
///
/// # Argument
/// * `multiboot_info_ptr` : pointeur multiboot2 permettant la cartographie de la mémoire pour être utilisé par le noyau ensuite.
#[unsafe(no_mangle)]
pub extern "C" fn kernel_start(multiboot_info_ptr : u64) -> ! {   
    crate::disp_debug!("Kernel starts at 0x{:x}", core::ptr::addr_of!(__kernel_start) as u64);
    crate::disp_debug!("Kernel ends at 0x{:x}", core::ptr::addr_of!(__kernel_end) as u64);

    // Vérification du format du pointeur multiboot.
    if !multiboot_info_ptr.is_multiple_of(8) {
        crate::disp_warning!("Unaligned multiboot2 pointer.");
    }

    if multiboot_info_ptr == 0 {
        panic!("The multiboot2 info pointer is NULL.");
    }

    crate::disp_debug!("Multiboot2 info pointer = 0x{:x}", multiboot_info_ptr);
    crate::disp_debug!("Physical memory offset = 0x{:x}", PHYSICAL_MEMORY_OFFSET);

    // Fabriquation de la carte de la mémoire à partir du pointeur multiboot_info
    let boot_info = unsafe { 
        BootInformation::load((multiboot_info_ptr + PHYSICAL_MEMORY_OFFSET) as *const BootInformationHeader)
            .expect("Failed to load Multiboot2 boot info.")
    };
    let memory_map_tag = unsafe {
        let tag = boot_info.memory_map_tag().expect("Memory map tag required.");
        &*(tag as *const multiboot2::MemoryMapTag)
    };

    // Création des alloueurs mémoire.
    crate::disp_info!("Frame allocator and kernel mapper initialization.");
    unsafe { memory::init(memory_map_tag) };

    // Allocation de la zone du tas.
    crate::disp_info!("Kernel heap initialization.");
    allocator::init_heap()
        .expect("Heap initialization failed.");

    // On initialise le reste du kernel
    init();

    // On affiche les informations d'aligement de la mémoire.
    crate::disp_debug!("User stack will start at 0x{:x}.", user_mode::USER_STACK_START);
    crate::disp_debug!("User stack will end at 0x{:x}.", user_mode::USER_STACK_START+user_mode::USER_STACK_SIZE as u64-1);
    crate::disp_debug!("User pages starts at 0x{:x}.", memory::USER_PAGES_START);
    crate::disp_debug!("User pages ends at 0x{:x}.", memory::USER_PAGES_END);
    crate::disp_debug!("Kernel pages starts at 0x{:x}.", memory::KERNEL_PAGES_START);
    crate::disp_debug!("Kernel pages ends at 0x{:x}.", allocator::HEAP_START-1);
    crate::disp_debug!("Heap starts at 0x{:x}.", allocator::HEAP_START);
    crate::disp_debug!("Head ends at 0x{:x}.", allocator::HEAP_START+allocator::HEAP_SIZE-1);

    crate::disp_info!("Syscalls initialization.");
    unsafe {
        syscalls::init_syscalls(
            get_selectors().get_kernel_code_selector(), 
            get_selectors().get_kernel_data_selector(),
            get_selectors().get_user_code_selector(), 
            get_selectors().get_user_data_selector()
        );
    }

    crate::disp_info!("Creating new memory mapper for user pages.");
    let (user_pml4_frame, mut user_mapper) = user_mode::create_user_page_table();

    crate::disp_debug!("Reading elf64 executable file.");
    static ELF_DATA: &elf::AlignedElfBinary<[u8]> = &elf::AlignedElfBinary(*include_bytes!("../user/hello_world/hello"));
    let entry_point = unsafe { elf::load_elf(&ELF_DATA.0, &mut user_mapper) };

    crate::disp_info!("Loading user PML4.");
    unsafe {
        use x86_64::registers::control::{Cr3Flags, Cr3};
        Cr3::write(user_pml4_frame, Cr3Flags::empty());
    }

    crate::disp_debug!("Elf entry is at 0x{:x}", entry_point.as_u64());

    crate::disp_info!("Getting to ring 3.");
    user_mode::enter_user_space(&mut user_mapper, entry_point);

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

/// Fonction d'initialisation des composantes du noyau comme la table d'interrutions et les ports x86_64
fn init() {
    crate::disp_info!("GDT initialization.");
    gdt::init();

    crate::disp_info!("IDT initialization.");
    interrupts::init_idt();

    crate::disp_info!("PICS initialization.");
    unsafe { interrupts::PICS.lock().initialize() };

    crate::disp_info!("SSE initialization.");
    unsafe { init_sse(); }

    crate::disp_info!("Enabling CPU interruption.");
    x86_64::instructions::interrupts::enable();
}

/// Fonction d'initialisation des instructions SSE.
unsafe fn init_sse() {
    // On active FXSAVE/FXRSTOR et les exceptions SIMD dans CR4
    let mut cr4 = Cr4::read();
    cr4.insert(Cr4Flags::OSFXSR);
    cr4.insert(Cr4Flags::OSXMMEXCPT_ENABLE);
    Cr4::write(cr4);

    // On s'assure que la copie du coprocesseur est désactivée et le monitoring activé dans CR0
    let mut cr0 = Cr0::read();
    cr0.remove(Cr0Flags::EMULATE_COPROCESSOR); // Effacer EM
    cr0.insert(Cr0Flags::MONITOR_COPROCESSOR); // Définir MP
    Cr0::write(cr0);
}

/// Fonction d'arrêt du processeur en fonction du processeur.<br>
// TODO le faire fonctionner pour d'autres architectures que le x86_64.
fn hlt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
