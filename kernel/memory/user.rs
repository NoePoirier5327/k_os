//! Module mémoire dédié à l'utilisateur.

use x86_64::{VirtAddr, structures::paging::{FrameAllocator, PageTable, PhysFrame}};
use crate::kernel::Kernel;

/// Créer une nouvelle pml4 utilisateur à partir de la pml4 kernel.
pub fn new_user_pml4() -> PhysFrame {
    let phys_mem_offset = Kernel::on_instance().physical_memory_offset();

    // On alloue la nouvelle pml4
    let new_pml4_frame = Kernel::with_frame_allocator(|frame_allocator| {
        frame_allocator
            .allocate_frame()
            .expect("No more memory left to allocate user's pml4.")
    });

    let new_pml4: &mut PageTable = unsafe {
        let phys_addr = new_pml4_frame.start_address().as_u64();
        let virt_addr = VirtAddr::new(phys_addr + phys_mem_offset);
        &mut *virt_addr.as_mut_ptr::<PageTable>()
    };

    // On la néttoie
    new_pml4.zero();

    // On récupère la pml4 kernel.
    let kernel_pml4_frame = Kernel::on_instance().get_pml4_frame();
    let kernel_pml4 = unsafe {
        let phys_addr = kernel_pml4_frame.start_address().as_u64();
        let virt_addr = VirtAddr::new(phys_addr + phys_mem_offset);
        &*virt_addr.as_ptr::<PageTable>()
    };

    // On copie la partie haute de la table kernel dans la table utilisateur.
    for i in 256..512 {
        new_pml4[i] = kernel_pml4[i].clone();
    }

    // La partie basse sera peuplée lors du chargement du binaire
    // et de la pile utilisateur.

    new_pml4_frame
}
