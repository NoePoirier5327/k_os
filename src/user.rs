mod user_mode;

use x86_64::VirtAddr;

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
        user_mode::enter_user_mode(selectors, entry_point, stack_top);
    }
}


fn test_user_function() {
    loop {

    }
}
