//! Implémentation d'un allocateur de type liste chaînée. <br>
//! Code tirée du tutoriel de Philipp Opermann.

use super::{align_up, Locked};
use alloc::alloc::{GlobalAlloc, Layout};
use core::{mem, ptr};


/// Type représentant la tête de la liste contenant les ressources allouées.
pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    /// Constructeur d'une liste vide.
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }

    /// Initialise la liste à l'adresse du début du tas.
    ///
    /// # Arguments
    /// * `heap_start` : Adresse de début du tas.
    /// * `heap_size` : Taille du tas auquel on se réfert.
    ///
    /// # Safety
    /// L'appelant doit garantir que l'adresse de tas est valide et qu'il n'est pas utilisé. <br>
    /// Cette méthode ne doit être appelé qu'une seule fois.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.add_free_region(heap_start, heap_size);
        }
    }

    /// Alloue une région mémoire en tête de liste.
    ///
    /// # Arguments
    /// * `addr` : Adresse à laquelle allouer la mémoire.
    /// * `size` : Taille de la zone à allouer.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // On vérifie la validitée de la zone à allouer
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        // On créer la nouvelle tête et la place à sa place.
        let mut new_head = ListNode::new(size);
        new_head.next = self.head.next.take();

        // On place la nouvelle tête dans le tas.
        let node_ptr = addr as *mut ListNode;
        unsafe {
            node_ptr.write(new_head);
            self.head.next = Some(&mut *node_ptr);
        }
    }

    /// Détermine si la région en paramètre est acceptable pour l'allocation. <br>
    /// Pour cela, on vérifie si la région a une taille suffisamment grande pour 
    /// l'allocation qu'on veut faire et vérifie que la région peut être diviser
    /// en deux pour avoir une partie utilisée et une partie libre.
    ///
    /// # Arguments
    /// * `region` : région dont l'acceptabilitée est à tester.
    /// * `size` : taille de la région en paramètre.
    /// * `align` : alignement de la région.
    ///
    /// # Return
    /// Renvoie l'adresse de début d'allocation si elle est acceptable.
    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        // On vérifie que la région est assez grande.
        if alloc_end > region.end_addr() {
            return Err(());
        }

        // On vérifie que le reste de la région est libre.
        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            return Err(());
        }

        // region suitable for allocation
        Ok(alloc_start)
    }

    /// Cherche une région mémoire correspondante à la taille et l'alignement en paramètre.
    ///
    /// # Arguments
    /// * `size` : taille de la zone à chercher.
    /// * `align` : alignement de la zone recherchée.
    ///
    /// # return
    /// None si aucune région correspondante n'est trouvée.
    /// Sinon, tuple du noeud dans lequel la zone mémoire est accessible et son adresse de début.
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        let mut current = &mut self.head;

        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                current = current.next.as_mut().unwrap();
            }
        }

        // no suitable region found
        None
    }

    /// Ajuste le layout en paramètre pour que la région allouée
    /// correspondante puisse aussi stocker un noeud.
    ///
    /// # Return
    /// Renvoie la taille ajustée du noeud et son alignement sous 
    /// forme de tuple (size, align).
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("ERROR : Adjusting alignment failed.")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}


/// Implémentation du trait GlobalAlloc pour la liste chaînée pour pouvoir l'utiliser
/// avec la crate alloc tirée de la std.
unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                unsafe {
                    allocator.add_free_region(alloc_end, excess_size);
                }
            }
            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = LinkedListAllocator::size_align(layout);

        unsafe { self.lock().add_free_region(ptr as usize, size) }
    }
}


/// Noeud courant de la liste chaînée correspondant aux blocks mémoire allouées.
struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    /// Constructeur d'un noeud de la liste d'une certaine taille.<br>
    /// Par défaut, le noeud n'est relié à aucun autre.
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    /// Accesseur de l'adresse de départ du blocks courant.
    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    /// Accesseur de l'adresse de fin du blocks courant.
    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}
