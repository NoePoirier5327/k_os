//! Implémentation d'un alloueur sur le tas. <br>
//! Code tiré du tutoriel de Philipp Opermann

pub mod bump;
pub mod linked_list;
pub mod fixed_size_block;

use fixed_size_block::FixedSizeBlockAllocator;
use crate::memory::types::{InclusivePageRange, Page, PageFlags, VirtAddr, MemoryAllocationError};
use crate::arch::hal::memory::{FrameAllocator, FrameAllocatorTrait, Mapper, MapperTrait, MemoryLayout};
use crate::arch::MEMORY_LAYOUT;


#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());


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
    mapper: &mut Mapper,
    frame_allocator: &mut FrameAllocator
) ->Result<(), MemoryAllocationError> {
    let heap_start = MEMORY_LAYOUT.get_kernel_heap_start();
    let heap_end = MEMORY_LAYOUT.get_kernel_heap_end();
    let heap_size = (heap_end - heap_start + 1) as usize;

    let page_range = {
        let heap_start_vaddr = VirtAddr::new(heap_start);
        let heap_end_vaddr = VirtAddr::new(heap_end);
        let heap_start_page = Page::new(heap_start_vaddr);
        let heap_end_page = Page::new(heap_end_vaddr);
        InclusivePageRange::new(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = match frame_allocator.allocate_frame() {
            Some(frame) => {
                //crate::disp_debug!("Page 0x{:x} | Frame 0x{:x}", page.get_start_address().as_u64(), frame.get_start_address().as_u64());
                frame
            },
            None => return Err(MemoryAllocationError::FrameAllocationFailed)
        };

        let flags = PageFlags::PRESENT | PageFlags::WRITABLE;

        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?
        };
    }

    // On initialise correctement l'allocateur.
    unsafe {
        ALLOCATOR.lock().init(heap_start as usize, heap_size);
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
