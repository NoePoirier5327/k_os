//! Fichier contenant des procédures de gestion de la mémoire du noyau comme le paging.<br>
//! code majoritairement tiré du tutoriel de Philipp Opermann.

use x86_64::structures::paging::PageTable;
use x86_64::VirtAddr;
use x86_64::registers::control::Cr3;

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
pub unsafe fn active_level_4_table(physical_memory_offset : VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    // L'adresse virtuelle est l'adresse physique + l'offset
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    &mut *page_table_ptr
}
