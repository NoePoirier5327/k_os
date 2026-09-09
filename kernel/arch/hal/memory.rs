//! Module de gestion des allocations mémoires globales.

use crate::memory::types::{PhysFrame, PhysAddr, Page, PageFlags, VirtAddr};

/// Frame allocator global indépendant du matériel cible.
pub trait FrameAllocator {
    /// Alloue un cadre physique et le renvoie
    /// Si erreur quelconque, renvoie None.
    fn allocate_frame(&mut self) -> Option<PhysFrame>;

    /// Désalloue le cadre physique en paramètre.
    fn deallocate_frame(&mut self, frame: PhysFrame);
}

/// Mapper de page dans l'alloueur de cadre au dessus.
pub trait Mapper {
    /// Mappe une page virtuelle vers une frame physique, la page est mapper selon un mot de droit
    /// géré par le type PageFlags.
    ///
    /// # Safety
    /// L'appelant doit garantir que la plage d'adresses à mapper n'écrase pas le noyau.
    unsafe fn map_to(
        &mut self,
        page: Page,
        frame: PhysFrame,
        flags: PageFlags,
        allocator: &mut dyn FrameAllocator
    ) -> Result<(), &'static str>;

    /// Supprime le mappage de la page en paramètre et renvoie son cadre physique.
    /// Si quelconque erreur, renvoie un message d'erreur.
    ///
    /// # Safety
    /// L'appelant doit s'assurer qu'il ne supprime pas le mappage d'une page noyau.
    unsafe fn unmap(
        &mut self,
        page: Page
    ) -> Result<PhysFrame, &'static str>;

    /// Traduit l'adresse virtuelle en paramètre en adresse physique.
    fn translate(
        &self,
        vaddr: VirtAddr
    ) -> Option<PhysAddr>;

    /// Défini ce mapper comme courant dans la mémoire.
    unsafe fn set_as_current(&self);
}
