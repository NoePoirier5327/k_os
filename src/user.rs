mod user_mode;

use x86_64::VirtAddr;
use x86_64::structures::paging::OffsetPageTable;
use crate::memory::BootInfoFrameAllocator;


/// Adresse de début de la pile utilisateur.
pub static USER_STACK_START : u64 = 0x0040_0000;

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

    // On place le point d'entrée de l'espace utilisateur dans une page utilisateur dédiée.
    let fn_adr = test_user_function as *const u8;
    unsafe {
        crate::memory::place_in_user_pages(mapper, frame_allocator, fn_adr, 128)
            .expect("Failed to map user space entry in user pages.");
    }
   
    // On prépare les arguments d'entrées en ring 3
    let stack_top = VirtAddr::new(USER_STACK_START+ USER_STACK_SIZE as u64);
    let selectors = crate::gdt::get_selectors(); 
    let entry_point = VirtAddr::from_ptr(crate::memory::USER_PAGES_START as *const ());

    crate::disp_info!("Swapping to ring 3.");
    unsafe {
        user_mode::enter_user_mode(
            selectors.get_user_code_selector().0,
            selectors.get_user_data_selector().0,
            entry_point.as_u64(), 
            stack_top.as_u64()
        );
    }
}

fn test_user_function() {
    let msg = "Welcome to KOs !";
    let msg_ptr = msg.as_ptr() as u64;
    let msg_len = msg.len() as u64;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 0,
            in("rsi") msg_ptr,
            in("rdx") msg_len,
            clobber_abi("sysv64"),
        );
    }

    loop {

    }
}
