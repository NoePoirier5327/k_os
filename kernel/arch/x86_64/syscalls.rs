//! Module de gestion de la bascule du mode utilisateur au mode noyau lors de syscall et de
//! l'initialisation du support de ces derniers pour l'architecture x86_64.

use crate::arch::hal::syscalls::SyscallInterface;
use crate::arch::x86_64::gdt::X86_64CPU_CONTEXT_INTERFACE;
use x86_64::VirtAddr;
use x86_64::registers::control::{Efer, EferFlags};
use x86_64::registers::model_specific::{KernelGsBase, LStar, Star, SFMask};
use core::arch::naked_asm;

/// Taille de la pile allouée aux appelles systèmes.
static SYSCALL_STACK_SIZE: usize = 16_384;

/// Pile dédiée aux appelles systèmes.
static mut SYSCALL_STACK: [u8; SYSCALL_STACK_SIZE] = [0u8; SYSCALL_STACK_SIZE];

/// Structure stockée dans la base Kernel GS.
/// Alignée sur 16 octets pour garantir des offsets précis.
#[repr(C, align(16))]
struct KernelGsData {
    pub kernel_stack: u64, // Offset 0x00
    pub _pad: u64,         // Offset 0x08
    pub user_rsp: u64,     // Offset 0x10
}

/// Données du Kernel GS
static mut KERNEL_GS_DATA: KernelGsData = KernelGsData {
    kernel_stack: 0,
    _pad: 0,
    user_rsp: 0,
};

pub static X86_64SYSCALL_INTERFACE: X86_64Syscalls = X86_64Syscalls;

/// Interface de gestion des syscalls pour l'architecture x86_64.
pub struct X86_64Syscalls;

impl SyscallInterface for X86_64Syscalls {
    fn init(&self) {
        crate::disp_info!("Enabling x86_64 syscalls (MSRs/GS base).");

        // On initialise la structure GS du noyau.
        let stack_top = unsafe { SYSCALL_STACK.as_mut_ptr().add(SYSCALL_STACK_SIZE) as u64 };
        unsafe {
            KERNEL_GS_DATA.kernel_stack = stack_top;
            KernelGsBase::write(VirtAddr::new(core::ptr::addr_of!(KERNEL_GS_DATA) as u64));
        }

        // On active les syscalls au niveau du registre EFER du cpu.
        unsafe {
            Efer::update(|flags| {
                flags.insert(EferFlags::SYSTEM_CALL_EXTENSIONS);
            });
        }

        // Le registre STAR indique au cpu quels segments charger lors de syscall/sysret.
        let selectors = X86_64CPU_CONTEXT_INTERFACE.get_selectors();
        match Star::write(
            selectors.get_user_code_selector(),
            selectors.get_user_data_selector(),
            selectors.get_kernel_code_selector(),
            selectors.get_kernel_data_selector()
        ) {
            Ok(_) => {},
            Err(e) => {
                panic!("Failed to load segments for syscall/sysret support {:?}", e);
            }
        }

        // Le registre LSTAR donne l'adresse du point d'entré des sycalls au cpu.
        LStar::write(VirtAddr::new(syscall_entry as *const() as u64));

        // Le registre SFMASK masque le drapeau d'interruption pour désactiver les interruptions 
        // le temps qu'on bascule sur la pile du noyau (évite les conditions de concurrence).
        SFMask::write(x86_64::registers::rflags::RFlags::INTERRUPT_FLAG);
    }

    fn update_kernel_stack(&self, stack_top: u64) {
        unsafe { KERNEL_GS_DATA.kernel_stack = stack_top; }
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
        syscall_dispatcher = sym crate::syscall::generic_syscall_dispatcher,
    );
}

