//! Module de gestion des architectures cibles pour le système d'exploitation.
// NOTE Ne supporte que le x86_64 pour le moment.

mod x86_64;

/// Module d'abstraction global, c'est à lui que seront fait les appelles.
pub mod hal;

// Définition des interface génériques d'appelle aux interruption
#[cfg(target_arch = "x86_64")]
pub use x86_64::interrupts::PIC_CONTROLLER as INTERRUPTION_CONTROLLER;

/// Détecte et renvoie l'architecture courante.
pub const CURRENT_ARCH: ArchType = 
    if cfg!(target_arch = "x86_64") { ArchType::x86_64 }
    else if cfg!(target_arch = "aarch64") { ArchType::aarch64 }
    else if cfg!(target_arch = "riscv64") { ArchType::riscv64 }
    else { ArchType::unknown };

/// Réprésente l'architecture cible sur laquelle tourne le noyau.
#[allow(non_camel_case_types)]
pub enum ArchType {
    x86_64,
    aarch64,
    riscv64,
    unknown
}

/// Renvoie l'adresse de début de l'espace mémoire utilisateur réservé dans l'architecture 
/// courante.
pub fn get_user_addr_range_start() -> u64 {
    match CURRENT_ARCH {
        ArchType::x86_64 => x86_64::USER_PAGES_START,
        ArchType::unknown => panic!("Unknown arch type target."),
        _ => panic!("Current architecture support not implemented yet.")
    }
}

/// Renvoie l'adresse de fin de l'espace mémoire réservé à l'utilisateur dans l'architecture
/// courante.
pub fn get_user_addr_range_end() -> u64 {
    match CURRENT_ARCH {
        ArchType::x86_64 => x86_64::USER_PAGES_END,
        ArchType::unknown => panic!("Unkown arch type target."),
        _ => panic!("Current architecture support not implemented yet.")
    }
}

/// Renvoie l'adresse de début de l'espace mémoire réservé au noyau dans l'architecture courante.
pub fn get_kernel_addr_range_start() -> u64 {
    match CURRENT_ARCH {
        ArchType::x86_64 => x86_64::KERNEL_PAGES_START,
        ArchType::unknown => panic!("Unkown arch type target."),
        _ => panic!("Current architecture support not implemented yet.")
    }
}

/// Renvoie l'adresse de fin de l'espace mémoire réservé au noyau dans l'architecture courante.
pub fn get_kernel_addr_range_end() -> u64 {
    match CURRENT_ARCH {
        ArchType::x86_64 => x86_64::KERNEL_PAGES_END,
        ArchType::unknown => panic!("Unknown arch type target."),
        _ => panic!("Current architecture support not implemented yet.")
    }
}

/// Met en veille l'exécution du processeur en fonction de l'architecture courante.
pub fn hlt_loop() -> ! {
    extern crate x86_64;
    use x86_64::instructions;

    loop {
        match CURRENT_ARCH {
            ArchType::x86_64 => instructions::hlt(),
            ArchType::unknown => panic!("Unknown arch type target."),
            _ => panic!("Current architecture support not implemented yet.")
        }
    }
}
