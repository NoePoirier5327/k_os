//! Module de gestion du mode utilisateur du cpu.

use core::arch::asm;

use x86_64::VirtAddr;
use x86_64::structures::paging::OffsetPageTable;
use crate::memory::BootInfoFrameAllocator;


/// Adresse de début de la pile utilisateur.
pub static USER_STACK_START : u64 = 0x003F_E000;

/// Taille de la pile utilisateur.
pub static USER_STACK_SIZE : usize = 4096 * 2;


/// Permet de démarrer le mode utilisateur dans le ring 3 du CPU.
///
/// # Arguments
/// * `mapper` : mapper de pages mémoire.
/// * `frame_allocator` : allocateur de pages mémoire.
pub fn enter_user_space(
    mapper : &mut OffsetPageTable,
    frame_allocator : &mut BootInfoFrameAllocator
) {
    // On alloue les pages correspondantes à la pile.
    unsafe {
        let start_adr = VirtAddr::new(USER_STACK_START);
        crate::memory::allocate_user_region(mapper, frame_allocator, start_adr, USER_STACK_SIZE)
            .expect("Failed to allocate user stack.");
    }
   
    // On prépare les arguments d'entrées en ring 3
    let stack_top = VirtAddr::new(USER_STACK_START+ USER_STACK_SIZE as u64);
    let selectors = crate::gdt::get_selectors(); 
    let entry_point = VirtAddr::from_ptr(crate::memory::USER_PAGES_START as *const ());

    crate::disp_info!("Swapping to ring 3.");
    unsafe {
        enter_user_mode(
            selectors.get_user_code_selector().0,
            selectors.get_user_data_selector().0,
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
    code_selector : u16,
    data_selector : u16,
    entry_point : u64, 
    user_stack_top : u64
) -> ! {    
    // RFLAGS : Bit 1 est toujours à 1. 
    // Bit 9 (0x200) correspond aux flags d'interruptions. 
    // On l'active pour que les interruptions matérielles restent actives en Ring 3.
    let rflags: u64 = 0x202; 

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
        ss = in(reg) data_selector,
        rsp = in(reg) user_stack_top,
        rflags = in(reg) rflags,
        cs = in(reg) code_selector,
        rip = in(reg) entry_point,
        in("ax") data_selector, // Charge ds, es, fs, gs via AX
        options(noreturn)
    );
}
