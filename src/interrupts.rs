//! Fichier contenant l'implémentation de la gestion de la IDT.<br>
//! Pour l'instant elle ne fonctionne qu'avec l'architecture x86-64. <br>
//! TODO Faire en sorte que ça fonctionne pour les autres.

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::println;
use lazy_static::lazy_static;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: InterruptStackFrame)
{
    println!("EXCEPTION: breakpoint\n{:#?}", stack_frame);
}
