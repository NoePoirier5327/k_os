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

    pub fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    /// Vérifie que l'adresse interne est alignée du nombre de bits en paramètre.
    pub fn is_aligned(&self, alignment: u64) -> bool {
        alignment.is_power_of_two() && (self.0 & (alignment - 1)) == 0
    }

    /// Renvoie l'adresse interne alignée vers le bas.
    ///
    /// # Panic
    /// Si alignment n'est pas une puissance de deux alors la fonction panique.
    pub fn align_down(&self, alignment: u64) -> Self {
        VirtAddr::new(align_down(self.0, alignment))
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
    /// Créer une nouvelle page de 4Kib avec l'adresse en paramètre comme adresse principale alignée
    /// vers le bas.
    pub fn new(addr: VirtAddr) -> Self {
        Self {
            start_address: addr.align_down(PAGE_SIZE as u64)
        }
    }

    pub fn get_start_address(&self) -> VirtAddr {
        self.start_address
    }
}

/// Structure de gestion de plage de page, la dernière page est inclue dans l'itérateur.
#[derive(Debug, Clone, Copy)]
pub struct InclusivePageRange {
    start: Page,
    end: Page
}

impl InclusivePageRange {
    pub fn new(start: Page, end: Page) -> Self {
        Self {
            start,
            end
        }
    }

    pub fn get_start_page(&self) -> Page {
        self.start
    }

    pub fn get_end_page(&self) -> Page {
        self.end
    }
}

impl core::iter::IntoIterator for InclusivePageRange {
    type Item = Page;
    type IntoIter = InclusivePageRangeIterator;

    fn into_iter(self) -> Self::IntoIter {
        InclusivePageRangeIterator {
            current: self.start,
            end: self.end,
            finished: false
        }
    }
}

/// Itérateur pour le type InclusivePageRange.
pub struct InclusivePageRangeIterator {
    current: Page,
    end: Page,
    finished: bool
}

impl InclusivePageRangeIterator {
    pub fn get_current_page(&self) -> Page {
        self.current
    }

    pub fn get_end_page(&self) -> Page {
        self.end
    }
}

impl Iterator for InclusivePageRangeIterator {
    type Item = Page;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None
        }

        let page = self.current;

        if page == self.end {
            self.finished = true;
        } else {
            let next_vaddr = VirtAddr::new(page.get_start_address().as_u64() + PAGE_SIZE as u64);
            self.current = Page::new(next_vaddr);
        }

        Some(page)
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

/// Format d'erreur dédié à l'allocation
#[derive(Debug)]
pub enum MemoryAllocationError {
    FrameAllocationFailed,
    MappingFailed,
    UnmappingFailed,
    UnalignedAddress,
    OutOfMemory
}

/// Aligne l'adresse en paramètre du nombre de bits représenté par l'alignement en paramètre.
///
/// # Panic
/// Si alignment n'est pas une puissance de deux la fonction panique.
pub const fn align_down(addr: u64, alignment: u64) -> u64 {
    if !alignment.is_power_of_two() {
        panic!("The alignment should be a power of two.");
    }

    addr & !(alignment - 1)
}
