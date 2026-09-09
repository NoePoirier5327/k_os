//! Contient les types génériques de gestion de la mémoire du kernel.
//! Tous les cadres et pages ont une taille fixe de 4Kib

/// Représente une adresse mémoire physique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(u64);

impl PhysAddr {
    pub fn new(addr: u64) -> Self {
        Self (addr)
    }

    /// Renvoie l'adresse interne sous forme de u64.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// Représente une adresse mémoire virtuelle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(u64);

impl VirtAddr {
    pub fn new(addr: u64) -> Self {
        Self (addr)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// Représente un cadre mémoire physique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysFrame {
    start_address: PhysAddr
}

impl PhysFrame {
    pub fn new(addr: PhysAddr) -> Self {
        Self {
            start_address: addr
        }
    }

    pub fn get_start_address(&self) -> PhysAddr {
        self.start_address
    }
}

/// Taille de page mappée dans la mémoire virtuelle
pub const PAGE_SIZE: usize = 4096usize;

/// Représente une page mappée dans la mémoire virtuelle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    start_address: VirtAddr
}

impl Page {
    pub fn new(addr: VirtAddr) -> Self {
        Self {
            start_address: addr
        }
    }

    pub fn get_start_address(&self) -> VirtAddr {
        self.start_address
    }
}

bitflags::bitflags! {
    /// Drapeaux de droits de page courante.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFlags: u32 {
        const PRESENT         = 1 << 0;
        const WRITABLE        = 1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const NO_EXEC         = 1 << 3;
    }
}
