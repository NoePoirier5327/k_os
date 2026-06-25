//! Implémentation d'un alloueur sur le tas. <br>
//! Code tiré du tutoriel de Philipp Opermann

pub mod bump;
pub mod linked_list;
pub mod fixed_size_block;

use x86_64::VirtAddr;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};
use fixed_size_block::FixedSizeBlockAllocator;


#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());


// Information de délimitation de la zone virtuelle du tas.
pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 5 * 1024 * 1024; // 5 MiB


/// Fonction cartographiant la zone mémoire du tas pour pouvoir y accéder plus tard. <br>
/// Les pages allouées au tas sont de 4Ko de taille.
///
/// # Arguments
/// * `mapper` : instance de l'outil de cartographie mémoire.
/// * `frame_allocator` : outil d'allocation de pages.
///
/// # Return
/// Renvoie soit rien si tout va bien, soit le détaille de l'erreur s'il y en a une.
pub fn init_heap(mapper: &mut impl Mapper<Size4KiB>, frame_allocator: &mut impl FrameAllocator<Size4KiB>) -> Result<(), MapToError<Size4KiB>> {
    crate::disp_info!("Heap initialization.");

    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE as u64 - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
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
