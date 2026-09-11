//! Module d'abstraction des instructions comme hlt_loop dépendantes de l'architecture cible.

#[cfg(target_arch = "x86_64")]
pub use x86_64_hlt_loop as hlt_loop;

/// Met en veille l'exécution du processeur en fonction de l'architecture courante.
pub fn x86_64_hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
