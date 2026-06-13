//! Module de gestion du mode utilisateur du cpu.

use core::arch::asm;
use x86_64::VirtAddr;
use x86_64::structures::gdt::SegmentSelector;
use crate::gdt::Selectors;


/// Force les deux bits de poids faible d'un sélecteur à 3 (Ring 3 Privilege).
fn prepare_user_selector(selector: SegmentSelector) -> u64 {
    (selector.0 | 3) as u64
}

/// Permet de démarrer le mode utilisateur dans le ring 3 du CPU.
pub fn enter_user_space() {
    // On crée une pile propre pour le mode utilisateur
    const USER_STACK_SIZE: usize = 4096 * 2;
    static mut USER_STACK: [u8; USER_STACK_SIZE] = [0; USER_STACK_SIZE];
    
    let stack_top = VirtAddr::from_ptr(&raw const USER_STACK) + USER_STACK_SIZE as u64;
    let selectors = crate::gdt::get_selectors();
    
    let entry_point = VirtAddr::from_ptr(test_user_function as *const ());

    crate::println!("INFO : Swapping to ring 3.");
    
    unsafe {
        enter_user_mode(selectors, entry_point, stack_top);
    }
}

/// Permet de basculer définitivement vers le ring 3.
/// 
/// # Safety
/// L'appelant doit s'assurer que la mémoire et/ou la pile de la fonction cible n'est pas mappée sur
/// le flag USER_ACCESSIBLE.
unsafe fn enter_user_mode(selectors: Selectors, entry_point: VirtAddr, user_stack_top: VirtAddr) -> ! {
    // On récupère les octets bruts des sélecteurs et on force le privilège à 3
    let data_selector = prepare_user_selector(selectors.get_user_data_selector());
    let code_selector = prepare_user_selector(selectors.get_user_code_selector());
    
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
        rsp = in(reg) user_stack_top.as_u64(),
        rflags = in(reg) rflags,
        cs = in(reg) code_selector,
        rip = in(reg) entry_point.as_u64(),
        in("ax") data_selector, // Charge ds, es, fs, gs via AX
        options(noreturn)
    );
}


fn test_user_function() {
    loop {

    }
}
