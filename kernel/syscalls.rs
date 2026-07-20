use core::arch::naked_asm;
use x86_64::registers::model_specific::{Star, LStar, SFMask, Efer, EferFlags, KernelGsBase};
use x86_64::structures::gdt::SegmentSelector;
use x86_64::VirtAddr;


// Codes de gestion des syscalls.
/// Syscall d'écriture dans la console.
const SYS_DISP : u64 = 0;

/// Syscall de changement de couleur du writer système.
const SYS_DISPCOLOR : u64 = 1;

// Codes de retour d'un syscall calqués sur les conventions linux.
/// Syscall demandé non implémenté.
const ENOSYS : i64 = 38;

/// Retour correct de syscall.
const ESUCCESS : i64 = 0;

/// Erreur d'execution
const EFAILED : i64 = -1;

/// Argument non conforme
const ARGERROR : i64 = -2;

/// Structure stockée dans la base Kernel GS.
/// Alignée sur 16 octets pour garantir des offsets précis.
#[repr(C, align(16))]
pub struct KernelGsData {
    pub kernel_stack: u64, // Offset 0x00
    pub _pad: u64,         // Offset 0x08
    pub user_rsp: u64,     // Offset 0x10
}

/// Pile dédiée aux appels systèmes du noyau (16 KiB)
static mut SYSCALL_STACK: [u8; 16384] = [0; 16384];

/// Données du Kernel GS
static mut KERNEL_GS_DATA: KernelGsData = KernelGsData {
    kernel_stack: 0,
    _pad: 0,
    user_rsp: 0,
};

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
    user_data_selector : SegmentSelector,
) {
    // On initialise la structure GS du noyau
    let stack_top = unsafe { SYSCALL_STACK.as_ptr().add(16384) as u64 };
    unsafe {
        KERNEL_GS_DATA.kernel_stack = stack_top;
        KernelGsBase::write(VirtAddr::new(core::ptr::addr_of!(KERNEL_GS_DATA) as u64));
    }

    // On active les syscalls au niveau du registre EFER du CPU.
    unsafe {
        Efer::update(|flags| {
            flags.insert(EferFlags::SYSTEM_CALL_EXTENSIONS)
        });
    }

    // STAR indique au CPU quels segments charger lors du syscall/sysret
    match Star::write(
        user_code_selector,
        user_data_selector,
        kernel_code_selector,
        kernel_data_selector
    ) {
        Ok(_) => {},
        Err(error_msg) => {
            panic!("Failed to load segments for syscall/sysret : {:?}", error_msg);
        }
    }

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
) -> i64 {
    match id {
        SYS_DISP => {
            let to_disp = unsafe {
                super::vga_buffer::extract_str_from_adr(arg1, arg2)
                    .expect("Failed to extract the desired string from the ram")
            };

            crate::print!("{}", to_disp);
            ESUCCESS
        }

        SYS_DISPCOLOR => {
            if arg1 > 15 || arg2 > 15 {
                return ARGERROR;
            }

            let ft_color = super::vga_buffer::Color::from_code_to_color(arg1 as u8);
            let bg_color = super::vga_buffer::Color::from_code_to_color(arg2 as u8);
            super::vga_buffer::set_writer_color(ft_color, bg_color);
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
        "push rbx",
        "push rcx",
        "push rsi", // Pour aligner la pile sur un multiple de 16

        // On appel le dispatcher pour lancer le syscall en paramètre.
        "mov rbx, rdi",
        "mov rdi, rax", // id
        "mov rcx, rbx", // arg3
        "call syscall_dispatcher",
        // le résultat du retour du syscall est dans le registre RAX.

        // On récupère l'état d'éxecution du thread utilisateur.
        "pop rsi",
        "pop rcx",
        "pop rbx",
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
