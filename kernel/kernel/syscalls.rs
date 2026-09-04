use core::arch::naked_asm;
use x86_64::registers::model_specific::{Star, LStar, SFMask, Efer, EferFlags, KernelGsBase};
use x86_64::structures::gdt::SegmentSelector;
use x86_64::VirtAddr;

// Codes de retour d'un syscall calqués sur les conventions linux.
/// Syscall demandé non implémenté.
const ENOSYS : i64 = 38;

/// Retour correct de syscall.
const ESUCCESS : i64 = 0;

/// Erreur d'execution
const EFAILED : i64 = -1;

/// Argument non conforme
const ARGERROR : i64 = -2;

/// Taille max des chaînes de caractères à afficher.
const MAX_SYSCALL_STR_SIZE: u64 = 2000;

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

/// Représente la fonction de gestion d'un appel système.
type SyscallFn = fn(u64,u64,u64) -> i64;

const SYSCALL_TABLE : [Option<SyscallFn>; 2] = [
    Some(sys_disp),
    Some(sys_dispcolor)
];

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

/// Modifie l'entrée kernel_stack du KERNEL_GS_DATA.
///
/// # Safety
/// la pile en paramètre doit être une pile kernel saine.
pub unsafe fn set_new_syscall_stack(stack_top: u64) {
    KERNEL_GS_DATA.kernel_stack = stack_top;
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
    match SYSCALL_TABLE.get(id as usize) {
        Some(Some(handler)) => handler(arg1, arg2, arg3),

        _ => {
            crate::disp_warning!("Syscall `0x{:x}` doesn't exist.", id);
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
        "mov rcx, rdx", // arg3
        "mov rdx, rsi", // arg2
        "mov rsi, rdi", // arg1
        "mov rdi, rax", // id
        "call {syscall_dispatcher}",
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
        syscall_dispatcher = sym syscall_dispatcher,
    );
}

/// Implémentation de l'appel système d'affichage sur le buffer vga.
///
/// # Arguments
/// - `msg` : pointeur vers le premier caractère du message à afficher.
/// - `msg_len` : taille du message à afficher.
/// - `dummy` : argument inutile.
fn sys_disp(msg: u64, msg_len: u64, _dummy: u64) -> i64 {
    // Empêche le déni de service lors de l'affichage.
    if msg_len > MAX_SYSCALL_STR_SIZE {
        return ARGERROR
    }

    // extract_str_from_adr vérifie que [msg; msg+msg_len] est dans les pages utilisateur.
    let to_disp = match unsafe {
        super::memory::extract_str_from_adr(msg, msg_len)
    } {
        Ok(s) => s,
        Err(_) => return ARGERROR
    };

    crate::print!("{}", to_disp);
    ESUCCESS
}

/// Implémentation de l'appel système de changement de couleur sur le buffer vga.
///
/// #Arguments
/// - `ft_color` : couleur du texte.
/// - `bg_color` : couleur de fond du texte.
/// - `dummy`    : argument inutile.
fn sys_dispcolor(ft_color: u64, bg_color: u64, _dummy: u64) -> i64 {
    if ft_color > 15 || bg_color > 15 {
        return ARGERROR;
    }

    crate::vga_buffer::set_writer_color(
        crate::vga_buffer::Color::from_code_to_color(ft_color as u8),
        crate::vga_buffer::Color::from_code_to_color(bg_color as u8)
    );

    ESUCCESS
}
