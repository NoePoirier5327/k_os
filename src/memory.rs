//! Fichier contenant des procédures de gestion de la mémoire du noyau comme le paging.<br>
//! code majoritairement tiré du tutoriel de Philipp Opermann.

use x86_64::structures::paging::page_table::FrameError;
use x86_64::structures::paging::PageTable;
use x86_64::structures::paging::PhysFrame;
use x86_64::structures::paging::Size4KiB;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::registers::control::Cr3;
use x86_64::VirtAddr;
use x86_64::PhysAddr;

use multiboot2::MemoryMapTag;
use multiboot2::MemoryArea;

// Adresses de début et fin du kernel.
extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
}

/// Structure d'un alloueur mémoire simple.
pub struct BootInfoFrameAllocator {
    memory_map : &'static MemoryMapTag,
    next : usize
}

impl BootInfoFrameAllocator {
    /// Fonction de création d'un alloueur à partit d'une carte de la mémoire de multiboot2.
    /// 
    /// # Argument
    /// * `memory_map` : carte de la mémoire obtenue via multiboot2.
    ///
    /// # Return
    /// Instance de l'alloueur.
    ///
    /// # Safety
    /// L'appelant doit garantir que la carte mémoire est valide.
    pub unsafe fn init(memory_map: &'static MemoryMapTag) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// Fonction de création d'un iterateur sur les frames utilisables.<br>
    /// Elle fait en sorte d'éviter les pages sur lesquelles le kernel est défini pour empêcher
    /// qu'elle réecrive par dessus.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // On récupère les zones disponibles de la mémoire.
        let regions = self.memory_map.memory_areas();
        let usable_regions = regions.iter().filter(|r| r.typ() == multiboot2::MemoryAreaType::Available);

        // On transforme chaques régions en une suite de bloque de 4Ko.
        usable_regions.flat_map(|r| {
            let frame_addresses = (r.start_address()..r.end_address()).step_by(4096);
            frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
        }).filter(|frame| {
            let start = unsafe { &__kernel_start as *const u8 as u64 };
            let end = unsafe { &__kernel_end as *const u8 as u64 };
            // On ne garde que les bloques mémoire qui ne sont pas dans le noyau
            frame.start_address().as_u64() < start || frame.start_address().as_u64() > end
        })
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

/// Fonction d'initialisation d'une nouvelle OffsetPageTable.
///
/// # Argument
/// * `physical_memory_offset` : offset d'accès aux pages mémoire.
///
/// # Return
/// nouvelle instance de OffsetPageTable de temps de vie static.
///
/// # Safety
/// L'appelant doit garantir que l'adresse physique complète est cartographiée sur la mémoire
/// virtuelle pour être accessible via l'offset en paramètre.<br>
/// De plus, cette fonction doit être appelée une seule foit pour éviter les références muables
/// `&mut` qui sont des comportements indéfinis pour rust.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

/// Fonction renvoyant un accès mutable sur la table mémoire active de niveau 4.
///
/// # Argument
/// * `physical_memory_offset` : offset d'accès aux pages mémoire.
///
/// # Safety
/// L'appelant doit s'assurer que l'adresse physique complète est configuré
/// sur l'adresse virtuelle de la page 4 en paramètre récursive.<br>
/// De plus, cette fonction doit seulement être appelée pour éviter les 
/// références mutables `&mut` (qui est un comportement indéfini sur rust).
unsafe fn active_level_4_table(physical_memory_offset : VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    // L'adresse virtuelle est l'adresse physique + l'offset
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    &mut *page_table_ptr
}

/// Fonction de passage d'une adresse virtuelle à une addresse physique.
///
/// # Arguments
/// * `addr` : adresse virtuelle de départ
/// * `physical_memory_offset` : décalage d'adresse physique de l'instance courante
///
/// # Return
/// Renvoie ou l'adresse physique cartographiée en mémoire ou None si non cartographiée.
/// 
/// # Safety
/// L'appelant doit garantir que l'adresse physique complète est bien cartographiée
/// en mémoire à l'offset en paramètre.
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

/// Fonction appelée par `translate_addr`, elle doit seulement être appelée par des bloques
/// unsafe. Philipp Opermann explique dans son tutoriel qu'elle est safe pour limiter le nombre
/// de portions de code unsafe dans le projet rust.
///
/// # Arguments
/// * `addr` : adresse virtuelle de départ
/// * `physical_memory_offset` : décalage d'adresse physique de l'instance courante
///
/// # Return
/// Renvoie ou l'adresse physique cartographiée en mémoire ou None si non cartographiée.
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    // On lit la table active de niveau 4 par le registre cr3
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [ addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index() ];
    let mut frame = level_4_table_frame;

    // On parcours les pages mémoire de plusieurs niveaux
    for &index in &table_indexes {
        // On convertit la portion de page courante en référence vers la table
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe {&*table_ptr};

        // On lit la l'entrée de la page et met à jour `frame`
        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }

    // On calcul l'adresse de la page voulue grâce à l'offset
    Some(frame.start_address() + u64::from(addr.page_offset()))
}
