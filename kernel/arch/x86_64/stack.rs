//! Module de gestion de pile de 16Kib aligné sur 16 octets.

use crate::kernel::Kernel;
use x86_64::VirtAddr;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags};

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

/// Alloue et mappe la pile utilisateur dans l'espace d'adressage courant.
///
/// # Arguments
/// - `page_count` : Nombre de pages de 4 KiB à allouer pour la pile (ex: 4 = 16 KiB).
///
/// # Return
/// Adresse virtuelle du haut de la pile (`user_stack_top`).
pub unsafe fn allocate_user_stack(page_count: usize) -> VirtAddr {
    // Top de pile fixé dans la zone haute autorisée pour le Ring 3
    let stack_top_vaddr = VirtAddr::new(0x0000_7FFF_FFFF_0000);
    let stack_size = (page_count * 4096) as u64;
    let stack_bottom_vaddr = stack_top_vaddr - stack_size;

    let start_page: Page = Page::containing_address(stack_bottom_vaddr);
    let end_page: Page = Page::containing_address(stack_top_vaddr - 1u64);

    let flags = PageTableFlags::PRESENT 
        | PageTableFlags::WRITABLE 
        | PageTableFlags::USER_ACCESSIBLE 
        | PageTableFlags::NO_EXECUTE;

    let virt_mem_offset = VirtAddr::new(Kernel::on_instance().physical_memory_offset());

    for page in Page::range_inclusive(start_page, end_page) {
        Kernel::with_frame_allocator(|frame_allocator| {
            let mut mapper = Kernel::on_instance().mapper();

            let frame = frame_allocator.allocate_frame().expect("Out of memory for user stack.");
            mapper.map_to(page, frame, flags, frame_allocator).unwrap().flush();

            // Nettoyage de la page via le HHDM
            let phys_addr = frame.start_address().as_u64();
            let kernel_vaddr = virt_mem_offset + phys_addr;
            core::ptr::write_bytes(kernel_vaddr.as_mut_ptr::<u8>(), 0, 4096);
        });
    }

    // On retourne l'adresse la plus haute (le sommet de la pile)
    stack_top_vaddr
}
