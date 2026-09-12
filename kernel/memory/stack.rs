//! Module de gestion de pile kernel.

use alloc::vec::Vec;
use super::types::{VirtAddr, PageFlags, Page, PhysFrame, InclusivePageRange, MemoryAllocationError};
use crate::arch::hal::memory::{FrameAllocatorTrait, MapperTrait};
use crate::kernel::Kernel;

/// Alloue un sommet de pile kernel.
pub struct KernelStackAllocator {
    next_slot: usize,
    free_slots: Vec<usize>
}

impl KernelStackAllocator {
    // Adresse dans le higher half réservée pour les piles kernel.
    const BASE_TOP: u64 = 0xFFFF_FF80_0000_0000u64;
    const STACK_SIZE: u64 = 16 * 1024; // 16Kib
    const GUARD_SIZE: u64 = 4 * 1024; // 4Kib
    const SLOT_SIZE: u64 = Self::STACK_SIZE + Self::GUARD_SIZE;

    pub fn new() -> Self {
        Self {
            next_slot: 0usize,
            free_slots: Vec::new()
        }
    }

    /// Alloue et renvoie un top_vaddr pour l'allocation de KernelStack16Kib.
    pub fn allocate_top(&mut self) -> VirtAddr {
        let slot = self.free_slots.pop().unwrap_or_else(|| {
            let current = self.next_slot;
            self.next_slot += 1;
            current
        });

        let offset = (slot as u64) * Self::SLOT_SIZE;
        VirtAddr::new(Self::BASE_TOP - offset)
    }

    /// Désalloue un top_vaddr.
    pub fn deallocate_top(&mut self, top_vaddr: VirtAddr) {
        let offset = Self::BASE_TOP - top_vaddr.as_u64();
        let slot = (offset / Self::SLOT_SIZE) as usize;
        self.free_slots.push(slot);
    }
}

/// Représente une pile d'appel kernel de 16Kib.
/// Utile pour le multithreading.
#[repr(C, align(16))]
pub struct KernelStack16Kib {
    top_vaddr: VirtAddr,
    start_page: Page,
    end_page: Page,
    allocated_frames: Vec<PhysFrame>,
}

impl KernelStack16Kib {
    /// Alloue et mappe une pile kernel dans le mapper kernel.
    pub unsafe fn allocate(
        top_vaddr: VirtAddr
    ) -> Result<Self, MemoryAllocationError> {
        // On vérifie l'alignement de top_vaddr
        if !top_vaddr.is_aligned(4096u64) {
            return Err(MemoryAllocationError::UnalignedAddress)
        }

        // Il faut 4 pages de 4096 octets pour une pile de 16Kib
        let page_count = 4usize;
        let stack_size = (page_count * 4096usize) as u64;
        let bottom_vaddr = VirtAddr::new(top_vaddr.as_u64() - stack_size);

        // On récupère les pages à allouer.
        let start_page = Page::new(bottom_vaddr);
        let end_page = Page::new(VirtAddr::new(top_vaddr.as_u64() - 1u64));

        // On place les flags kernel
        let flags = PageFlags::PRESENT
            | PageFlags::WRITABLE
            | PageFlags::NO_EXEC;

        let mut frames = Vec::with_capacity(page_count);
        let virt_mem_offset = VirtAddr::new(Kernel::on_instance().get_phys_mem_offset().as_u64());

        for page in InclusivePageRange::new(start_page, end_page) {
            // On alloue le cadre
            let frame = match Kernel::with_frame_allocator(|allocator| allocator.allocate_frame()) {
                Some(f) => f,
                None => {
                    Self::rollback_partial_allocation(start_page, &page, &mut frames);
                    return Err(MemoryAllocationError::OutOfMemory)
                }
            };

            // On le map
            let mapping_result = Kernel::with_memory(|allocator, mapper| {
                mapper.map_to(page, frame, flags, allocator)
            });

            // On gère l'erreur
            if mapping_result.is_err() {
                Kernel::with_frame_allocator(|allocator| allocator.deallocate_frame(frame));
                Self::rollback_partial_allocation(start_page, &page, &mut frames);
                return Err(MemoryAllocationError::MappingFailed)
            }

            // On nettoie le HHDM
            let virt_ptr = VirtAddr::new(virt_mem_offset.as_u64() + frame.get_start_address().as_u64()).as_mut_ptr::<u8>();
            core::ptr::write_bytes(virt_ptr, 0, 4096);

            // On l'envoie dans les cadres déjà alloué
            frames.push(frame);
        }

        Ok(
            Self {
                top_vaddr,
                start_page,
                end_page,
                allocated_frames: frames
            }
        )
    }

    /// Désalloue la pile kernel courante.
    ///
    /// # Safety
    /// Le mapper en paramètre doit être celui ayant servi à l'allocation.
    pub unsafe fn deallocate(
        &mut self,
    ) {
        Kernel::with_memory(|allocator, mapper| {
            for page in InclusivePageRange::new(self.start_page, self.end_page) {
                mapper.unmap(page).ok();
            }

            for frame in self.allocated_frames.drain(..) {
                allocator.deallocate_frame(frame);
            }
        });
    }

    /// Nettoie les pages pas entièrement allouées en cas d'échec de allocate.
    unsafe fn rollback_partial_allocation(
        start_page: Page,
        failed_page: &Page,
        frames: &mut Vec<PhysFrame>
    ) {
        Kernel::with_memory(|allocator, mapper| {
            // On démapp les pages qui ont posées problèmes
            if failed_page.get_start_address().as_u64() > start_page.get_start_address().as_u64() {
                let last_mapped_page = Page::new(VirtAddr::new(failed_page.get_start_address().as_u64() - 1u64));
                for page in InclusivePageRange::new(start_page, last_mapped_page) {
                    mapper.unmap(page).ok();
                }
            }

            // On désalloue les cadres ayant posées problème.
            for frame in frames.drain(..) {
                allocator.deallocate_frame(frame);
            }
        });
    }

    /// Renvoie l'adresse du haut de la pile courante.
    pub fn get_top_vaddr(&self) -> VirtAddr {
        self.top_vaddr
    }
}

impl Drop for KernelStack16Kib {
    fn drop(&mut self) {
        if !self.allocated_frames.is_empty() {
            panic!("KernelStack16Kib dropped without explicit deallocation !");
        }
    }
}

/// Représente une pile d'appel utilisateur de 16Kib.
/// Utile pour le multithreading.
pub struct UserStack16Kib {
    top_vaddr: VirtAddr,
    start_page: Page,
    end_page: Page,
    allocated_frames: Vec<PhysFrame>,
}

impl UserStack16Kib {
    /// Alloue et mappe une pile utilisateur dans le mapper specifié.
    ///
    /// # Safety
    /// top_vaddr doit être valide dans l'espace utilisateur et alignée sur 4096 octets.
    pub unsafe fn allocate(
        mapper: &mut dyn MapperTrait,
        top_vaddr: VirtAddr,
    ) -> Result<Self, MemoryAllocationError> {
        // On vérifie l'alignement de top_vaddr
        if !top_vaddr.is_aligned(4096u64) {
            return Err(MemoryAllocationError::UnalignedAddress)
        }

        let page_count = 4usize; // Il faut 4 pages pour une pile de 16Kib
        let stack_size = (page_count * 4096usize) as u64;
        let bottom_vaddr = VirtAddr::new(top_vaddr.as_u64() - stack_size);

        let start_page = Page::new(bottom_vaddr);
        let end_page = Page::new(VirtAddr::new(top_vaddr.as_u64() - 1u64));

        let flags = PageFlags::PRESENT
            | PageFlags::WRITABLE
            | PageFlags::USER_ACCESSIBLE
            | PageFlags::NO_EXEC;

        let mut frames = Vec::with_capacity(page_count);
        let virt_mem_offset = VirtAddr::new(Kernel::on_instance().get_phys_mem_offset().as_u64());

        for page in InclusivePageRange::new(start_page, end_page) {
            let frame = match Kernel::with_frame_allocator(|allocator| allocator.allocate_frame()) {
                Some(f) => f,
                None => {
                    Self::rollback_partial_allocation(mapper, start_page, &page, &mut frames);
                    return Err(MemoryAllocationError::OutOfMemory)
                }
            };

            // Mapping dans la PML4 utilisateur.
            let mapping_result = Kernel::with_frame_allocator(|allocator| {
                mapper.map_to(page, frame, flags, allocator)
            });

            // On vérifie le bon fonctionnement de l'allocation.
            if mapping_result.is_err() {
                Kernel::with_frame_allocator(|allocator| allocator.deallocate_frame(frame));
                Self::rollback_partial_allocation(mapper, start_page, &page, &mut frames);
                return Err(MemoryAllocationError::MappingFailed)
            }

            // Nettoyage de la page via HHDM
            let virt_ptr = VirtAddr::new(virt_mem_offset.as_u64() + frame.get_start_address().as_u64()).as_mut_ptr::<u8>();
            core::ptr::write_bytes(virt_ptr, 0, 4096);

            frames.push(frame);
        }

        Ok(
            Self {
                top_vaddr,
                start_page,
                end_page,
                allocated_frames: frames
            }
        )
    }

    /// Désalloue la pile de la pml4 et libère les frames physiques.
    ///
    /// # Safety
    /// Le mapper en paramètre doit être celui ayant servi à l'allocation.
    pub unsafe fn deallocate(&mut self, mapper: &mut dyn MapperTrait) {
        // On libère la mémoire dans la table des pages.
        for page in InclusivePageRange::new(self.start_page, self.end_page) {
            mapper.unmap(page).ok();
        }

        // On restitue les frames physiques au frame_allocator global.
        Kernel::with_frame_allocator(|frame_allocator| {
            for frame in self.allocated_frames.drain(..) {
                frame_allocator.deallocate_frame(frame);
            }
        });
    }

    /// Nettoie les pages pas entièrement mappées en cas d'échec de allocate.
    unsafe fn rollback_partial_allocation(
        mapper: &mut dyn MapperTrait,
        start_page: Page,
        failed_page: &Page,
        frames: &mut Vec<PhysFrame>
    ) {
        // On démapp les pages qui ont posées problème.
        if failed_page.get_start_address().as_u64() > start_page.get_start_address().as_u64() {
            let last_mapped_page = Page::new(VirtAddr::new(failed_page.get_start_address().as_u64() - 1u64));
            for page in InclusivePageRange::new(start_page, last_mapped_page) {
                mapper.unmap(page).ok();
            }
        }

        // On désalloue les frames qui ont posées problème.
        Kernel::with_frame_allocator(|allocator| {
            for frame in frames.drain(..) {
                allocator.deallocate_frame(frame);
            }
        });
    }

    /// Renvoie l'adresse du haut de la pile courante
    pub fn get_top_vaddr(&self) -> VirtAddr {
        self.top_vaddr
    }
}

impl Drop for UserStack16Kib {
    fn drop(&mut self) {
        if !self.allocated_frames.is_empty() {
            panic!("UserStack16Kib dropped without explicit deallocation!");
        }
    }
}

/// Alloue une pile d'appelle configuré pour les processus utilisateur dans le higher half
pub struct UserStackAllocator {
    next_slot: usize,
    free_slots: Vec<usize>,
}

impl UserStackAllocator {
    const BASE_TOP: u64 = 0x0000_7FFF_FFFF_0000u64; // Adresse canonique de x86_64
    const STACK_SIZE: u64 = 16 * 1024; // 16Kib
    const GUARD_SIZE: u64 = 4 * 1024; // 4 Kib
    const SLOT_SIZE: u64 = Self::STACK_SIZE + Self::GUARD_SIZE;

    pub fn new() -> Self {
        Self {
            next_slot: 0usize,
            free_slots: Vec::new()
        }
    }

    /// Calcul et renvoie un nouveau `top_vaddr` pour l'allocation de pile utilisateur pour les
    /// threads.
    pub fn allocate_top(&mut self) -> VirtAddr {
        let slot = self.free_slots.pop().unwrap_or_else(|| {
            let current = self.next_slot;
            self.next_slot += 1usize;
            current
        });

        let offset = (slot as u64) * Self::SLOT_SIZE;
        VirtAddr::new(Self::BASE_TOP - offset)
    }

    /// Recycles le slot correspondant à un top_vaddr libéré.
    pub fn deallocate_top(&mut self, top_vaddr: VirtAddr) {
        let offset = Self::BASE_TOP - top_vaddr.as_u64();
        let slot = (offset / Self::SLOT_SIZE) as usize;
        self.free_slots.push(slot);
    }
}
