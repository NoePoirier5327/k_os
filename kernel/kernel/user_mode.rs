//! Module de gestion du mode utilisateur du cpu.

use core::arch::asm;
use x86_64::VirtAddr;
use x86_64::structures::paging::{OffsetPageTable, PageTable, PhysFrame, FrameAllocator};
use x86_64::registers::control::Cr3;
use crate::kernel::Kernel;
use crate::arch::x86_64::gdt;


/// Adresse de début de la pile utilisateur.
pub static USER_STACK_START : u64 = 0x003F_E000;

/// Taille de la pile utilisateur.
pub static USER_STACK_SIZE : usize = 4096 * 2;

/// Créer un nouvel espace d'adressage vierge pour un processus utilisateur.
pub fn create_user_page_table() -> (PhysFrame, OffsetPageTable<'static>) {
    // On alloue une frame physique pour la nouvelle PML4
    let pml4_frame = Kernel::with_frame_allocator(|frame_allocator| {
        frame_allocator
            .allocate_frame()
            .expect("No more memory left to allocate new user pml4.")
    });

    let virt_mem_offset = VirtAddr::new(Kernel::on_instance().physical_memory_offset());

    // On calcule l'adresse virtuelle pour y accéder via le kernel
    let pml4_vaddr = virt_mem_offset + pml4_frame.start_address().as_u64();
    let pml4_ptr = pml4_vaddr.as_mut_ptr::<PageTable>();

    // On rempli la page de zéros afin d'effacer les données aléatoires en RAM.
    unsafe {
        core::ptr::write_bytes(pml4_ptr as *mut u8, 0, 4096);
    }
    let new_pml4 = unsafe { &mut *pml4_ptr };

    // On récupere l'ancienne table (actuelle) pour copier le noyau
    let (current_pml4_frame, _) = Cr3::read();
    let current_pml4_vaddr = virt_mem_offset + current_pml4_frame.start_address().as_u64();
    let current_pml4 = unsafe { &*current_pml4_vaddr.as_ptr::<PageTable>() };

    // On copie les entrées Kernel (256 à 511)
    // On laisse de 0 à 255 à zéro (espace utilisateur totalement vierge !)
    for i in 256..512 {
        new_pml4[i] = current_pml4[i].clone();
    }

    // On créer le Mapper associé à ce nouveau PML4
    let new_mapper = unsafe { OffsetPageTable::new(new_pml4, virt_mem_offset) };

    // On retourne le Frame physique (pour Cr3) et le Mapper (pour charger l'ELF)
    (pml4_frame, new_mapper)
}
