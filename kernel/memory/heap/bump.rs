//! Implémentation d'un alloueur mémoire de type bump,
//! il alloue une zone complête de taille précise pour chaques ressources
//! et déalloue toute la zone dés qu'on a besoin de déallouer la ressource. <br>
//! Code tiré du tutoriel de Philipp Opermann.

use alloc::alloc::{GlobalAlloc, Layout};
use super::{align_up, Locked};
use core::ptr;

/// Structure de l'allocateur de type Bump.
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    /// Créer une nouvelle instance de l'allocateur vide.
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initialise un allocateur sur la zone mémoire précisé en paramètre.
    ///
    /// # Arguments
    /// * `heap_start` : adresse virtuelle de début de la zone du tas.
    /// * `heap_size` : taille du tas à allouer.
    ///
    /// # Safety
    /// L'appelant doit s'assurer que la zone mémoire donnée est bien inutilisée. <br>
    /// De plus, il doit s'assurer que cette méthode n'est appelée qu'une seule fois.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}


/// Implémentation de GlobalAlloc pour pouvoir utiliser la crate alloc de la standard.
unsafe impl GlobalAlloc for Locked<BumpAllocator> {

    /// Méthode d'allocation mémoire d'une zone décrite en paramètre.
    ///
    /// # Arguments
    /// * `layout` : description de la zone mémoire à allouer.
    ///
    /// # Return
    /// Référence mutable sur la page allouée dans le tas, renvoie un pointeur null
    /// si on n'a plus de place accessible sur le tas.
    ///
    /// # Safety
    /// L'appelant doit s'assurer que la zone mémoire sur laquelle il souhaite allouer 
    /// une ressource n'est pas déjà utilisé par une autre ressource et que le tas est 
    /// correctement cartographié.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock();

        let alloc_start = align_up(bump.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };

        if alloc_end > bump.heap_end {
            ptr::null_mut() // out of memory
        } else {
            bump.next = alloc_end;
            bump.allocations += 1;
            alloc_start as *mut u8
        }
    }

    /// Méthode de déallocation d'une zone accessible au pointeur en paramètre. <br>
    /// Les paramètres sont données à la fonction car necessaire pour l'implémentation du trait
    /// GlobalAlloc mais inutile en réalitée pour un allocateur de type bump.
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump = self.lock();

        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next = bump.heap_start;
        }
    }
}
