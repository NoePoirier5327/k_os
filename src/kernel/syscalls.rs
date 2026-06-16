use core::arch::naked_asm;
use x86_64::registers::model_specific::{Star, LStar, SFMask};
use x86_64::structures::gdt::SegmentSelector;
use x86_64::VirtAddr;


// Codes de gestion des syscalls.
/// Syscall d'écriture dans la console.
const SYS_DISP : u64 = 0;

// Codes de retour d'un syscall calqués sur les conventions linux.
/// Syscall demandé non implémenté.
const ENOSYS : u64 = 38;

/// Retour correct de syscall.
const ESUCCESS : u64 = 0;


/// Initialise les appelles systèmes au niveau de l'assembleur.
///
/// # Arguments
/// * `kernel_code_selector` : point d'entrée, sur la gdt, du segment de code sur lequel est les syscalls.
/// * `kernel_data_selector` : point d'entrée, sur la gdt, du segment de donnée sur lequel est les syscalls.
/// * `user_code_selector` : segment de code, sur la gdt, sur lequel revenir après un syscall.
/// * `user_data_selector` : segment de donnée, sur la gdt, sur lequel revenir après un syscall. 
///
/// # Safety
/// L'appelant doit d'assurer que les ségments de code auquels il accède sont bien définis dans la
/// gdt.
pub unsafe fn init_syscalls(
    kernel_code_selector : SegmentSelector,
    kernel_data_selector : SegmentSelector,
    user_code_selector : SegmentSelector,
    user_data_selector : SegmentSelector
) {
    // STAR indique au CPU quels segments charger lors du syscall/sysret
    Star::write(
        user_code_selector,
        user_data_selector,
        kernel_code_selector,
        kernel_data_selector
    ).unwrap();

    // LSTAR donne l'adresse du point d'entrée assembleur au CPU
    LStar::write(VirtAddr::new(syscall_entry as *const () as u64));

    // SFMASK masque le drapeau d'interruption pour désactiver les interruptions 
    // le temps qu'on bascule sur la pile du noyau (évite les conditions de concurrence).
    SFMask::write(x86_64::registers::rflags::RFlags::INTERRUPT_FLAG);
}

/// Dispatcher d'appels système, appel les fonctions kernels correspondantes au syscall courant.
///
/// # Arguments
/// * `id` : identifiant de l'appel système demandé.
/// * `arg1` : 1er argument du syscall.
/// * `arg2` : 2nd argument du syscall.
/// * `arg3` : 3ème argument du syscall.
///
/// # Return
/// Renvoie un code permettant de connaître le resultat de l'appel.
///
/// # Safety
#[no_mangle]
unsafe extern "sysv64" fn syscall_dispatcher(
    id : u64,
    arg1 : u64,
    arg2 : u64,
    arg3 : u64
) -> u64 {
    match id {
        SYS_DISP => {
            let to_disp = unsafe {
                super::vga_buffer::extract_str_from_adr(arg1, arg2)
                    .expect("Failed to extract the desired string from the ram")
            };

            crate::print!("{}", to_disp);
            ESUCCESS
        }

        _ => {
            crate::disp_warning!("Le syscall {} n'existe pas.", id);
            ENOSYS
        }
    }
}

/// Echange le contexte du CPU pour executer le syscall demandé.
///
/// # Safety
#[unsafe(naked)]
unsafe extern "sysv64" fn syscall_entry() {
    naked_asm!(
        // swapgs échange le registre GS de l'utilisateur avec le GS du noyau.
        "swapgs",
        "mov gs:[0x10], rsp", // Sauvegarde le RSP (pile) de l'utilisateur
        "mov rsp, gs:[0x00]", // Charge le RSP (pile) du noyau

        // On sauvegarde l'état d'execution du thread dans la pile utilisateur.
        "push rbp",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "push rdi",
        "push rsi",

        // On appel le dispatcher pour lancer le syscall en paramètre.
        "mov rdi, rax", // id
        "mov rsi, rbx", // arg1
        "mov rax, rdx",
        "mov rdx, rcx", // arg2
        "mov rcx, rax", // arg3
        "call syscall_dispatcher",
        // le résultat du retour du syscall est dans le registre RAX.

        // On récupère l'état d'éxecution du thread utilisateur.
        "pop rsi",
        "pop rdi",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rbp",

        // Restauration de la pile utilisateur et retour
        "mov rsp, gs:[0x10]",
        "swapgs",
        "sysretq",
    );
}
