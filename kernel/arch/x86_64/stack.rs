//! Module de gestion de pile de 16Kib aligné sur 16 octets.

#[repr(C, align(16))]
pub struct Stack16Kib([u8; 16_384]);

impl Stack16Kib {
    /// Renvoie une nouvelle pile vide.
    pub const fn empty() -> Self {
        Self ([0u8; 16_384])
    }

    /// Renvoie l'adresse du haut de la pile courante.
    pub fn get_top(&self) -> u64 {
        self.0.as_ptr() as u64 + 16_384u64
    }
}
