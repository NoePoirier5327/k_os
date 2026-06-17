pub mod scheduler;
pub mod message;
pub mod vga_buffer;
pub mod syscalls;

use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};


/// Fonction d'initialisation des composantes du noyau comme la table d'interrutions et les ports x86_64
pub fn init() {
    crate::disp_info!("GDT initialization.");
    crate::gdt::init();

    crate::disp_info!("IDT initialization.");
    crate::interrupts::init_idt();

    crate::disp_info!("PICS initialization.");
    unsafe { crate::interrupts::PICS.lock().initialize() };

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
