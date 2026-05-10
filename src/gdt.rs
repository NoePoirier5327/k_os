//! Implémentation d'un TSS pour la gestion de double fault.<br>
//! TODO Implémenter d'autres architectures que le x86_64.

use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};
use x86_64::structures::gdt::SegmentSelector;
use lazy_static::lazy_static;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    /// S'occupe de trouver une zone mémoire saine et accessible pour replacer
    /// le pointeur de pile.
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: u64 = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE as usize] = [0; STACK_SIZE as usize];

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            stack_start + STACK_SIZE
            //stack_end
        };
        tss
    };
}

/// Type gérant les segments mémoire pour le déplacement de pile.
struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector
}

lazy_static! {
    /// S'occupe de déplacer le pointeur de pile lors de double fault.
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code_selector, tss_selector })
    };
}

/// Fonction de chargement de la GDT pour l'échange de pile lors de double
/// fault notamment causer par un stack overflow.
pub fn init() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, Segment};
    
    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
