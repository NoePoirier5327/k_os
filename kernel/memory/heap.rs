//! Implémentation d'un alloueur sur le tas. <br>
//! Code tiré du tutoriel de Philipp Opermann

pub mod bump;
pub mod linked_list;
pub mod fixed_size_block;

use fixed_size_block::FixedSizeBlockAllocator;
use crate::memory::types::{InclusivePageRange, Page, PageFlags, VirtAddr, MemoryAllocationError};
use crate::arch::hal::memory::{FrameAllocator, Mapper};


#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());


// Information de délimitation de la zone virtuelle du tas.
pub const HEAP_START: usize = 0xFFFF_9000_0000_0000;
pub const HEAP_SIZE: usize = 5 * 1024 * 1024; // 5 MiB


/// Fonction cartographiant la zone mémoire du tas pour pouvoir y accéder plus tard. <br>
/// Les pages allouées au tas sont de 4Ko de taille.
///
/// # Arguments
/// * `mapper`: mapper mémoire kernel pour l'allocation du tas.
/// * `frame_allocator`: alloueur de page kernel pour l'allocation du tas.
///
/// # Return
/// Renvoie soit rien si tout va bien, soit le détaille de l'erreur s'il y en a une.
pub fn init_heap(
    mapper: &mut dyn Mapper,
    frame_allocator: &mut dyn FrameAllocator
) ->Result<(), MemoryAllocationError> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start.as_u64() + HEAP_SIZE as u64 - 1u64;
        let heap_start_page = Page::new(heap_start);
        let heap_end_page = Page::new(VirtAddr::new(heap_end));
        InclusivePageRange::new(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MemoryAllocationError::FrameAllocationFailed)?;

        let flags = PageFlags::PRESENT | PageFlags::WRITABLE;

        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?
        };
    }

    // On initialise correctement l'allocateur.
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}


/// Renvoie l'adresse en paramètre alignée vers le haut avec le reste des adresses. <br>
/// Le paramètre align doit être une puissance de 2.
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}


/// Interface de spin::Mutex pour permettre l'implémentation de trait pour cette dernière.
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    /// Constructeur d'un Mutex pour une ressource en paramètre.
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    /// Méthode de bloquage du verrou mémoire sur la donnée de l'interface.
    pub fn lock(&self) -> spin::MutexGuard<'_, A> {
        self.inner.lock()
    }
}
