//! Fichier contenant l'implémentation de la gestion de la IDT.<br>
//! Pour l'instant elle ne fonctionne qu'avec l'architecture x86-64. <br>
//! Le code est majoritairement du tutoriel de Philipp Opermann.
// TODO Faire en sorte que ça fonctionne pour d'autres architectures.

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use pic8259::ChainedPics;
use spin::Lazy;
use crate::{println, print};
use crate::gdt;
use crate::hlt;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });


#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard
}

impl InterruptIndex {
    /// Fonction de transtypage du type InterruptIndex en u8.
    ///
    /// # Return
    /// u8 correspondant à l'instance de InterruptIndex courante.
    fn to_u8(self) -> u8 {
        self as u8
    }

    /// Fonction de transtypage du type InterruptIndex en usize.
    ///
    /// # Return
    /// usize correspondant au InterruptIndex courant.
    fn to_usize(self) -> usize {
        usize::from(self.to_u8())
    }
}

pub static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();

    // On référence la fonction de gestion de breakpoint.
    idt.breakpoint.set_handler_fn(breakpoint_handler);

    // On référence la fonction de gestion de double_fault et sa fonction de swap de pile.
    unsafe {
        idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
    }

    // On référence la fonction de gestion du timer.
    idt[InterruptIndex::Timer.to_u8()].set_handler_fn(timer_interrupt_handler);

    // On référence la fonction de gestion des entrées claviers.
    // ATTENTION, pour l'instant on ne supporte que les ports ps2.
    // Cependant, les ports USB sont émulés en ps2 donc pas de problème pour le moment.

    idt[InterruptIndex::Keyboard.to_u8()].set_handler_fn(keyboard_interrupt_handler);

    idt.page_fault.set_handler_fn(page_fault_handler);

    idt
});

pub fn init_idt() {
    IDT.load();
}

/// Fonction gérant les interruptions de séquences qui ne nécessite pas de code d'erreur.<br>
/// Elle affiche le message d'erreur puis rend la main au système.
///
/// # Argument
/// * `stack_frame` : message d'erreur à afficher.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: breakpoint\n{:#?}", stack_frame);
}

/// Fonction gérant les interruptions de séquences avec code d'erreur.<br>
/// Elle appelle la panic avant de redonner la main au système.
///
/// # Arguments
/// * `stack_frame` : message d'erreur à envoyer à la panic.
/// * `_error_code` : code d'erreur correspondant à l'erreur en paramètre.
extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    panic!("EXCEPTION: double fault\nError code : {:#?}\n{:#?}", _error_code, stack_frame);
}

/// Fonction de gestion du timer.
///
/// # Argument
/// * `stack_frame` : message d'interruption du timer.
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    //print!(".");
    unsafe {
      PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.to_u8());
    }
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
            }
        }
    }

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.to_u8());
    }
}

/// Fonction de gestion des dépassements d'accès mémoire aussi appelé page fault.
///
/// # Arguments
/// * `stack_frame` : Message d'erreur correspondant à la portion de la pile touchée.
/// * `error_code` : code d'erreur correspondant au dépassement
extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: page fault");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt();
}
