//! Module de gestion de pile de 16Kib aligné sur 16 octets.

use alloc::vec::Vec;
use x86_64::{VirtAddr, structures::paging::{FrameDeallocator, FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size4KiB}};
use crate::tasker::{TaskerResult, TaskerError};
use crate::kernel::Kernel;

/// Représente une pile d'appel kernel de 16Kib.
/// Utile pour le multithreading.
#[repr(C, align(16))]
pub struct KernelStack16Kib([u8; 16_384]);

impl KernelStack16Kib {
    /// Renvoie une nouvelle pile vide.
    pub const fn empty() -> Self {
        Self ([0u8; 16_384])
    }

    /// Renvoie l'adresse du haut de la pile courante.
    pub fn get_top(&self) -> u64 {
        self.0.as_ptr() as u64 + 16_384u64
    }
}

/// Représente une pile d'appel utilisateur de 16Kib.
/// Utile pour le multithreading.
pub struct UserStack16Kib {
    top_vaddr: VirtAddr,
    start_page: Page,
    end_page: Page,
    allocated_frames: Vec<PhysFrame<Size4KiB>>,
}

impl UserStack16Kib {
    /// Alloue et mappe une pile utilisateur dans le mapper specifié.
    ///
    /// # Safety
    /// top_vaddr doit être valide dans l'espace utilisateur et alignée sur 4096 octets.
    pub unsafe fn allocate(
        mapper: &mut OffsetPageTable<'static>,
        top_vaddr: VirtAddr,
    ) -> TaskerResult<Self> {
        // On vérifie l'alignement de top_vaddr
        if !top_vaddr.is_aligned(4096u64) {
            return Err(TaskerError::InvalidAddress)
        }

        let page_count = 4usize; // Il faut 4 pages pour une pile de 16Kib
        let stack_size = (page_count * 4096usize) as u64;
        let bottom_vaddr = top_vaddr - stack_size;

        let start_page = Page::containing_address(bottom_vaddr);
        let end_page = Page::containing_address(top_vaddr - 1u64);

        let flags = PageTableFlags::PRESENT
            | PageTableFlags::WRITABLE
            | PageTableFlags::USER_ACCESSIBLE
            | PageTableFlags::NO_EXECUTE;

        let mut frames = Vec::with_capacity(page_count);
        let virt_mem_offset = VirtAddr::new(Kernel::on_instance().physical_memory_offset());

        for page in Page::range_inclusive(start_page, end_page) {
            let frame = match Kernel::with_frame_allocator(|allocator| allocator.allocate_frame()) {
                Some(f) => f,
                None => {
                    Self::rollback_partial_allocation(mapper, start_page, &page, &mut frames);
                    return Err(TaskerError::OutOfMemory)
                }
            };

            // Mapping dans la PML4 utilisateur.
            let map_result = Kernel::with_frame_allocator(|allocator| {
                mapper.map_to(page, frame, flags, allocator)
            });

            // On vérifie le bon fonctionnement de l'allocation.
            if let Ok(mapper_flush) = map_result {
                mapper_flush.flush();
            } else {
                Kernel::with_frame_allocator(|allocator| allocator.deallocate_frame(frame));
                Self::rollback_partial_allocation(mapper, start_page, &page, &mut frames);
                return Err(TaskerError::MappingFailed)
            }

            // Nettoyage de la page via HHDM
            let virt_ptr = (virt_mem_offset + frame.start_address().as_u64()).as_mut_ptr::<u8>();
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
    pub unsafe fn deallocate(&mut self, mapper: &mut OffsetPageTable<'static>) {
        // On libère la mémoire dans la table des pages.
        for page in Page::range_inclusive(self.start_page, self.end_page) {
            if let Ok((_, mapper_flush)) = mapper.unmap(page) {
                mapper_flush.flush();
            }
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
        mapper: &mut OffsetPageTable<'static>,
        start_page: Page,
        failed_page: &Page,
        frames: &mut Vec<PhysFrame<Size4KiB>>
    ) {
        // On démapp les pages qui ont posées problème.
        if failed_page > &start_page {
            let last_mapped_page = Page::containing_address(failed_page.start_address() - 1u64);
            for page in Page::range_inclusive(start_page, last_mapped_page) {
                if let Ok((_, mapper_flush)) = mapper.unmap(page) {
                    mapper_flush.flush();
                }
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
    pub fn get_top(&self) -> u64 {
        self.top_vaddr.as_u64()
    }
}
