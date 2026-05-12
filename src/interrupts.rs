//! Fichier contenant l'implémentation de la gestion de la IDT.<br>
//! Pour l'instant elle ne fonctionne qu'avec l'architecture x86-64. <br>
// TODO Faire en sorte que ça fonctionne pour d'autres architectures.

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin;
use crate::{println, print};
use crate::gdt;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });


#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
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

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // On référence la fonction de gestion de breakpoint.
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        // On référence la fonction de gestion de double_fault et sa fonction de swap de pile.
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        // On référence la fonction de gestion du timer.
        idt[InterruptIndex::Timer.to_usize()].set_handler_fn(timer_interrupt_handler);

        idt
    };
}

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
    print!(".");
    unsafe {
      PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.to_u8());
    }
}

