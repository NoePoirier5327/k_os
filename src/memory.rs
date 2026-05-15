//! Fichier contenant des procédures de gestion de la mémoire du noyau comme le paging.<br>
//! code majoritairement tiré du tutoriel de Philipp Opermann.

use x86_64::structures::paging::PageTable;

/// Adresse virtuelle de la page 4 récursive.
const RECURSIVE_P4_ADDR: u64 = 0xffff_ffff_ffff_f000;

/// Fonction renvoyant un accès mutable sur la table mémoire active de niveau 4.
///
/// # Safety
/// L'appelant doit s'assurer que l'adresse physique complète est configuré
/// sur l'adresse virtuelle de la page 4 récursive.<br>
/// De plus, cette fonction doit seulement être appelée pour éviter les 
/// références mutables `&mut` (qui est un comportement indéfini sur rust).
pub unsafe fn active_level_4_table() -> &'static mut PageTable {
    let page_table_ptr = RECURSIVE_P4_ADDR as *mut PageTable;
    &mut *page_table_ptr
}
