//! Module de gestion du mode utilisateur du cpu.

use core::arch::asm;
use x86_64::VirtAddr;
use x86_64::structures::paging::{OffsetPageTable, PageTable, PhysFrame, FrameAllocator};
use x86_64::registers::control::Cr3;
use crate::kernel::Kernel;


/// Adresse de début de la pile utilisateur.
pub static USER_STACK_START : u64 = 0x003F_E000;

/// Taille de la pile utilisateur.
pub static USER_STACK_SIZE : usize = 4096 * 2;


/// Permet de démarrer le mode utilisateur dans le ring 3 du CPU.
///
/// # Arguments
/// * `entry_point` : pointer du mode utilisateur.
pub fn enter_user_space(
    entry_point : VirtAddr
) {
    // On alloue la pile utilisateur.
    let mut mapper = Kernel::on_instance().mapper();
    Kernel::with_frame_allocator(|frame_allocator| {
        let start_adr = VirtAddr::new(USER_STACK_START);
        unsafe {
            let _ = super::memory::allocate_user_region(
                &mut mapper,
                frame_allocator,
                start_adr,
                USER_STACK_SIZE
            );
        }
    });
   
    // On prépare les arguments d'entrées en ring 3
    let stack_top = VirtAddr::new(USER_STACK_START + USER_STACK_SIZE as u64);

    unsafe {
        enter_user_mode(
            entry_point.as_u64(), 
            stack_top.as_u64()
        );
    }
}

/// Permet de basculer définitivement vers le ring 3.
/// 
/// # Safety
/// L'appelant doit s'assurer que la mémoire et/ou la pile de la fonction cible n'est pas mappée sur
/// le flag USER_ACCESSIBLE.
unsafe fn enter_user_mode(
    entry_point : u64, 
    user_stack_top : u64
) -> ! {
    // RFLAGS : Bit 1 est toujours à 1. 
    // Bit 9 (0x200) correspond aux flags d'interruptions. 
    // On l'active pour que les interruptions matérielles restent actives en Ring 3.
    let rflags: u64 = 0x202; 

    let selectors = super::gdt::get_selectors();
    let user_code_selector: u64 = selectors.get_user_code_selector().0 as u64;
    let user_data_selector: u64 = selectors.get_user_data_selector().0 as u64;

    asm!(
        // On charge les registres de segment de données de l'utilisateur.
        // Le registre SS sera géré automatiquement par iretq.
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",

        // On forge la Stack Frame requise par iretq dans l'ordre inverse du dépilage :
        "push {ss}",        // [rsp + 32] Segment de pile utilisateur
        "push {rsp}",       // [rsp + 24] Pointeur de pile utilisateur (RSP)
        "push {rflags}",    // [rsp + 16] Registre d'état RFLAGS
        "push {cs}",        // [rsp + 8]  Segment de code utilisateur
        "push {rip}",       // [rsp + 0]  Pointeur d'instruction cible (RIP)

        // Enfin, on saute vers le ring 3.
        "iretq",

        // Entrées de l'assembleur
        ss = in(reg) user_data_selector,
        rsp = in(reg) user_stack_top,
        rflags = in(reg) rflags,
        cs = in(reg) user_code_selector,
        rip = in(reg) entry_point,
        in("ax") user_data_selector, // Charge ds, es, fs, gs via AX
        options(noreturn)
    );
}

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
