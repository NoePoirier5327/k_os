pub mod scheduler;


/// Fonction d'initialisation des composantes du noyau comme la table d'interrutions et les ports x86_64
pub fn init() {
    crate::gdt::init();

    crate::print!("IDT initialization ");
    crate::interrupts::init_idt();
    crate::print!("(OK)\n");

    crate::print!("PICS initialization ");
    unsafe { crate::interrupts::PICS.lock().initialize() };
    crate::print!("(OK)\n");

    crate::print!("Enabling CPU interruption ");
    x86_64::instructions::interrupts::enable();
    crate::print!("(OK)\n");
}
