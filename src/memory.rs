//! Fichier contenant des procédures de gestion de la mémoire du noyau comme le paging.<br>
//! code majoritairement tiré du tutoriel de Philipp Opermann.

use x86_64::structures::paging::page_table::FrameError;
use x86_64::structures::paging::{
    Page,
    Size4KiB,
    Mapper,
    FrameAllocator,
    PageTableFlags,
    PageTable,
    OffsetPageTable,
    PhysFrame
};
use x86_64::registers::control::Cr3;
use x86_64::{VirtAddr, PhysAddr};
use multiboot2::MemoryMapTag;
use crate::user::{USER_STACK_START, USER_STACK_SIZE};

// Adresses de début et fin du kernel.
extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
}

/// Adresse de début des pages utilisateur.<br>
/// Elles vont de USER_STACK_START + USER_STACK_SIZE à 0x8000_0000 -1.
pub static USER_PAGES_START : u64 = USER_STACK_START + USER_STACK_SIZE as u64;
pub static KERNEL_PAGES_START : u64 = 0x8000_0000;

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
        let usable_regions = regions.iter().filter(|r| {
            r.typ() == multiboot2::MemoryAreaType::Available
        });

        usable_regions.flat_map(|r| {
            let frame_addresses = (r.start_address()..r.end_address()).step_by(4096);
            frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
        }).filter(|frame| {
            // Utilisation sécurisée de addr_of! pour éviter de créer des références invalides
            let start = core::ptr::addr_of!(__kernel_start) as u64;
            let end = core::ptr::addr_of!(__kernel_end) as u64;
            
            // On ne garde que les blocs mémoire qui ne sont pas dans le noyau
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

/// Syscall d'allocation de memoire avec le privilège utilisateur.
///
/// # Arguments
/// * `page` : page à allouer à l'utilisateur.
/// * `mapper` : interface de cartographie de la mémoire.
/// * `frame_allocator` : allocateur de frame physique en mémoire.
///
/// # Safety
/// L'appelant y accède uniquement par un syscall
// TODO trouver un emplacement plus adapté à cette fonction dans l'architecture.
pub unsafe fn allocate_user_page(
    page: Page<Size4KiB>, 
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>
) -> Result<(), &'static str> {
    
    // On demande une frame brute à l'allocateur.
    let frame = frame_allocator.allocate_frame()
        .ok_or("ERROR : No more physical memory left.")?;

    // Flags de sécuritée pour le mappage
    let flags = PageTableFlags::PRESENT 
              | PageTableFlags::WRITABLE 
              | PageTableFlags::USER_ACCESSIBLE;

    // On map la page souhaitée avec les flags spécifiés.
    unsafe {
        let _ = mapper.map_to(page, frame, flags, frame_allocator);
    }

    Ok(())
}

/// Place l'adresse de la fonction en paramètre dans une zone accessible à l'utilisateur.
///
/// # Arguments
/// * `mapper` : instance du mapper de page.
/// * `frame_allocator` : instance de l'allocateur d'emplacement.
/// * `fn_adr` : adresse à placer dans une page accessible à l'utilisateur.
/// * `fn_size` : taille de la fonction à copier.
///
/// # Safety
/// L'appelant doit s'assurer que la zone à laquelle il accède est bien défini en mémoire.
// TODO trouver un emplacement plus adapté dans l'architecture à cette fonction.
pub unsafe fn place_in_user_pages(
    mapper : &mut OffsetPageTable,
    frame_allocator : &mut BootInfoFrameAllocator,
    fn_adr : *const u8,
    fn_size : usize
) -> Result<(), &'static str> {
    // On créer une page utilisateur dans la zone dédiée
    let user_page : Page<Size4KiB> = Page::containing_address(VirtAddr::new(USER_PAGES_START));
    allocate_user_page(user_page, mapper, frame_allocator)?;

    // On copie l'adresse de la fonction dans cette nouvelle page.
    let src = fn_adr;
    let dest = USER_PAGES_START as *mut u8;
    core::ptr::copy_nonoverlapping(src, dest, fn_size);

    Ok(())
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
