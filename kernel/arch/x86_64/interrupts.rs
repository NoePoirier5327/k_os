//! Fichier contenant l'implémentation de la gestion de la IDT

use x86_64::structures::idt::{HandlerFunc, InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use pic8259::ChainedPics;
use spin::{Lazy, Mutex};
use super::gdt::X86_64CPU_CONTEXT_INTERFACE;
use crate::memory::types::VirtAddr;
use crate::tasker::Tasker;
use core::arch::naked_asm;
use crate::{println, print};
use crate::arch::hal::interrupts::{InterruptionType, InterruptionController};

/// Interface globale du contrôleur d'interruption pic.
/// Contient un mutex en interne sur la gestion de sa logique.
pub static PIC_CONTROLLER: Mutex<PicController> = Mutex::new(unsafe { PicController::new() });

/// Offsets du driver PIC
const PIC_1_OFFSET: u8 = 32;
const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// Instance du contrôleur d'interruptions pour x86_64.
pub struct PicController {
    pics: ChainedPics
}

impl PicController {
    /// Instancie un nouveau contrôleur pic.
    /// 
    /// # Safety
    /// L'appelant doit s'assurer qu'il n'y a qu'une seule instance de contrôleur par coeur cpu.
    pub const unsafe fn new() -> Self {
        Self {
            pics: ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)
        }
    }
}

impl InterruptionController for PicController {
    fn init(&mut self) {
        // On initialise le contrôleur
        unsafe { self.pics.initialize() };

        // Puis, on charge la table d'interruption
        IDT.load();
        unsafe {
            configure_pit();
        }
    }

    fn enable(&mut self, i_type: InterruptionType) {
        let irq_line = i_type.to_irq_line();
        unsafe {
            let [mut mask1, mut mask2] = self.pics.read_masks();
            if irq_line < 8 {
                mask1 &= !(1 << irq_line);
            }
            else {
                mask2 &= !(1 << irq_line);
            }
            self.pics.write_masks(mask1, mask2);
        }
    }

    fn disable(&mut self, i_type: InterruptionType) {
        let irq_line = i_type.to_irq_line();
        unsafe {
            let [mut mask1, mut mask2] = self.pics.read_masks();
            if irq_line < 8 {
                mask1 |= 1 << irq_line;
            }
            else {
                mask2 |= 1 << irq_line;
            }
            self.pics.write_masks(mask1, mask2);
        }
    }

    fn end_of_interrupt(&mut self, i_type: InterruptionType) {
        let irq = i_type.to_u8();
        unsafe {
            self.pics.notify_end_of_interrupt(irq);
        }
    }
}

/// Execute la fonction en paramètre en désactivant les interruptions processeur.
pub fn without_interrupts<R>(f: impl FnOnce() -> R) -> R {
    x86_64::instructions::interrupts::without_interrupts(|| f())
}

impl InterruptionType {
    /// Implémentation de la transformation d'interruption vers un index dans la table d'interruption.
    fn to_u8(&self) -> u8 {
        match self {
            InterruptionType::Timer => PIC_1_OFFSET,
            InterruptionType::Keyboard => PIC_1_OFFSET + 1
        }
    }

    /// Renvoie la ligne irq brut du pic (0 à 15)
    fn to_irq_line(&self) -> u8 {
        match self {
            InterruptionType::Timer => 0u8,
            InterruptionType::Keyboard => 1u8
        }
    }
}

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();

    // On référence la fonction de gestion de breakpoint.
    idt.breakpoint.set_handler_fn(breakpoint_handler);

    // On référence la fonction de gestion de double_fault et sa fonction de swap de pile.
    unsafe {
        idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(super::gdt::DOUBLE_FAULT_IST_INDEX);
    }

    // On référence la fonction de gestion du timer.
    let timer_handler: HandlerFunc = unsafe { core::mem::transmute(timer_interrupt_handler as *const ()) };
    idt[InterruptionType::Timer.to_u8()].set_handler_fn(timer_handler);

    // On référence la fonction de gestion des entrées claviers.
    // ATTENTION, pour l'instant on ne supporte que les ports ps2.
    // Cependant, les ports USB sont émulés en ps2 donc pas de problème pour le moment.

    idt[InterruptionType::Keyboard.to_u8()].set_handler_fn(keyboard_interrupt_handler);

    idt.page_fault.set_handler_fn(page_fault_handler);
    idt.invalid_opcode.set_handler_fn(invalid_iterruption_code_handler);

    idt
});

/// Fonction de configuration des interruptions processeurs. <br>
/// Cadence les interruptions à 10 ms.
///
/// # Safety
/// Ne doit être appelée qu'une seule fois.
unsafe fn configure_pit() {
    let frequency = 100; // 100 Hz
    let divisor = 1193182 / frequency;

    use x86_64::instructions::port::Port;

    // Port de commande du PIT
    let mut cmd_port = Port::new(0x43);
    // Port de données du Canal 0 du PIT
    let mut data_port = Port::new(0x40);

    // 0x36 = Mode 3 (Square Wave Generator), Canal 0, accès bas/haut octet
    cmd_port.write(0x36u8);
    
    // Envoyer le diviseur (octet bas puis octet haut)
    data_port.write((divisor & 0xFF) as u8);
    data_port.write(((divisor >> 8) & 0xFF) as u8);
}

/// Fonction gérant les interruptions de séquences qui ne nécessite pas de code d'erreur.<br>
/// Elle affiche le message d'erreur puis rend la main au système.
///
/// # Argument
/// * `stack_frame` : message d'erreur à afficher.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    crate::disp_exception!("breakpoint\n{:#?}", stack_frame);
}

/// Fonction gérant les interruptions de séquences avec code d'erreur.<br>
/// Elle appelle la panic avant de redonner la main au système.
///
/// # Arguments
/// * `stack_frame` : message d'erreur à envoyer à la panic.
/// * `_error_code` : code d'erreur correspondant à l'erreur en paramètre.
extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    crate::disp_exception!("double fault.");
    panic!("Error code : {:#?}\n{:#?}", _error_code, stack_frame);
}

/// Gère les interruptions processeur.
/// Echange les threads courants pour supporter le multiprocessus.
#[unsafe(naked)]
extern "C" fn timer_interrupt_handler() {
    naked_asm!(
        // On sauvegarde les registres généraux du thread sortant
        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rsi",
        "push rdi",
        "push rbp",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // On passe le RSP actuel en 1er argument (RDI dans l'ABI System V)
        "mov rdi, rsp",
        "call {handle_switch}",

        // On applique le nouveau RSP renvoyé dans RAX par handle_switch
        "mov rsp, rax",

        // On s'acquitte de l'interruption timer auprès du pic8259 maître.
        "mov al, 0x20",
        "out 0x20, al",

        // On restaure les registres généraux du thread entrant
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rbp",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",

        // On quitte l'interruption (restaure RIP, CS, RFLAGS, RSP, SS)
        "iretq",
        handle_switch = sym Tasker::handle_switch,
    );
}

/// Fonction de gestion des interruptions clavier.<br>
/// Elle redonne la main au système après l'interruption.
///
/// # Argument
/// * `stack_frame` : message d'interruption du clavier.
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, PS2Keyboard, HandleControl, ScancodeSet1};
    use x86_64::instructions::port::Port;
    use spin::Mutex;

    static KEYBOARD: Lazy<spin::Mutex<PS2Keyboard<layouts::Azerty, ScancodeSet1>>> = Lazy::new(|| {
        Mutex::new(PS2Keyboard::new(
            ScancodeSet1::new(),
            layouts::Azerty,
            HandleControl::Ignore,
        ))
    });

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            };
        };
    }

    PIC_CONTROLLER.lock().end_of_interrupt(InterruptionType::Keyboard);
}

/// Fonction de gestion des dépassements d'accès mémoire aussi appelé page fault.
///
/// # Arguments
/// * `stack_frame` : Message d'erreur correspondant à la portion de la pile touchée.
/// * `error_code` : code d'erreur correspondant au dépassement
extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    use x86_64::registers::control::Cr2;

    crate::disp_exception!("page fault.");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    crate::arch::hlt_loop();
}

/// Fonction de gestion d'interruption inconnue.
///
/// # Arguments
/// * `stack_frame` : portion de la mémoire dans laquelle l'erreur s'est produite
extern "x86-interrupt" fn invalid_iterruption_code_handler(stack_frame: InterruptStackFrame) {
    crate::disp_exception!("Invalid interruption code found.");
    panic!("{:#?}", stack_frame);
}

/// Implémentation d'une stack frame x86_64 pour le changement de contexte x86_64.
#[repr(C)]
struct X86_64InterruptionStackFrame {
    // Registres généraux empilés manuellement (dans l'ordre inverse des PUSH)
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,

    // Empilés automatiquement par le CPU lors de l'interruption
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

/// Interface de création de stack frame kernel.
/// Renvoie la nouvelle adresse du registre rsp.
///
/// # Safety
/// L'appelant doit assurer que le kernel_stack_top pointe vers une pile saine et accessible.
/// De même pour exec_entry_point.
pub unsafe fn new_kernel_interruption_stack_frame(
    kernel_stack_top: VirtAddr,
    exec_entry_point: u64,
) -> VirtAddr {
    // On récupère les segments de données kernel.
    let selectors = X86_64CPU_CONTEXT_INTERFACE.get_selectors();
    let kernel_cs = selectors.get_kernel_code_selector().0 as u64;
    let kernel_ss = selectors.get_kernel_data_selector().0 as u64;

    // On détermine la taille de la stack frame.
    let stack_frame_size = core::mem::size_of::<X86_64InterruptionStackFrame>() as u64;
    
    // Puis l'instancie sous forme de pointeur.
    let stack_top = VirtAddr::new(kernel_stack_top.as_u64() - stack_frame_size);

    // On aligne la frame initiale sur la stack_top.
    let stack_frame_ptr = &mut *(stack_top.as_u64() as *mut X86_64InterruptionStackFrame);
    *stack_frame_ptr = X86_64InterruptionStackFrame {
        // Normalement obtenus par iretq.
        cs: kernel_cs,
        ss: kernel_ss,
        rsp: kernel_stack_top.as_u64(),
        rip: exec_entry_point,
        rflags: 0x202,

        // Registres d'exécutions initiaux.
        rax: 0, rbx: 0, rcx: 0, rdx: 0,
        rsi: 0, rdi: 0, rbp: 0, r8: 0,
        r9: 0, r10: 0, r11: 0, r12: 0,
        r13: 0, r14: 0, r15: 0
    };

    stack_top
}

/// Interface de création de stack_frame utilisateur.
/// Renvoie la nouvelle adresse pour le registre rsp.
///
/// # Safety
/// L'appelant doit assurer que le kernel_stack_top pointe vers une pile saine et accessible.
/// De même pour user_stack_top et exec_entry_point.
pub unsafe fn new_user_interruption_stack_frame(
    user_stack_top: VirtAddr,
    kernel_stack_top: VirtAddr,
    exec_entry_point: u64
) -> VirtAddr {
    // Récuperation des segments accessible à l'utilisateur.
    let selectors = X86_64CPU_CONTEXT_INTERFACE.get_selectors();
    let user_cs = selectors.get_user_code_selector().0 as u64;
    let user_ss = selectors.get_user_data_selector().0 as u64;

    // On calcul la taille de la stack frame en mémoire.
    let stack_frame_size = core::mem::size_of::<X86_64InterruptionStackFrame>() as u64;

    // On créer détermine l'adresse du pointeur vers la nouvelle stack frame.
    let stack_top = VirtAddr::new(kernel_stack_top.as_u64() - stack_frame_size);

    // On instancie la frame initiale qu'on aligne sur la stack_top.
    let stack_frame_ptr = &mut *(stack_top.as_u64() as *mut X86_64InterruptionStackFrame);
    *stack_frame_ptr = X86_64InterruptionStackFrame {
        // Normalement obtenus par iretq.
        cs: user_cs,
        ss: user_ss,
        rsp: user_stack_top.as_u64(),
        rip: exec_entry_point,
        rflags: 0x202,

        // Registres d'exécutions initiaux.
        rax: 0, rbx: 0, rcx: 0, rdx: 0,
        rsi: 0, rdi: 0, rbp: 0, r8:  0,
        r9:  0, r10: 0, r11: 0, r12: 0, 
        r13: 0, r14: 0, r15: 0
    };

    stack_top
}
