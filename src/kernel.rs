pub mod scheduler;
pub mod message;

/// Fonction d'initialisation des composantes du noyau comme la table d'interrutions et les ports x86_64
pub fn init() {
    crate::disp_info!("GDT initialization.");
    crate::gdt::init();

    crate::disp_info!("IDT initialization.");
    crate::interrupts::init_idt();

    crate::disp_info!("PICS initialization.");
    unsafe { crate::interrupts::PICS.lock().initialize() };

    crate::disp_info!("Enabling CPU interruption.");
    x86_64::instructions::interrupts::enable();
}
