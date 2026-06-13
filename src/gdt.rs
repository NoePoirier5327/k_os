//! Implémentation d'un TSS pour la gestion de double fault.<br>
//! Code majoritairement tiré du tutoriel de Philipp Opermann.
// TODO Implémenter d'autres architectures que le x86_64.

use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use spin::Lazy;
use crate::print;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// S'occupe de trouver une zone mémoire saine et accessible pour replacer
/// le pointeur de pile.
static TSS: Lazy<TaskStateSegment> = Lazy::new(|| {
    let mut tss = TaskStateSegment::new();

    // Pile saine utilisée lors de Double Fault
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
        const STACK_SIZE: u64 = 4096 * 5;
        static mut STACK: [u8; STACK_SIZE as usize] = [0; STACK_SIZE as usize];

        let stack_start = VirtAddr::from_ptr(&raw const STACK);
        stack_start + STACK_SIZE
    };

    // Pile utilisée pour les interruptions en ring 3.
    tss.privilege_stack_table[0] = {
        const STACK_SIZE: u64 = 4096 * 5;
        static mut KERNEL_RSP0_STACK: [u8; STACK_SIZE as usize] = [0; STACK_SIZE as usize];
        let stack_start = VirtAddr::from_ptr(&raw const KERNEL_RSP0_STACK);
        stack_start + STACK_SIZE
    };

    tss
});

/// Type gérant les segments mémoire pour le déplacement de pile.
#[derive(Debug, Clone, Copy)]
pub struct Selectors {
    kernel_code_selector: SegmentSelector,
    kernel_data_selector: SegmentSelector,
    user_code_selector: SegmentSelector,
    user_data_selector: SegmentSelector,
    tss_selector: SegmentSelector
}

impl Selectors {
    pub fn get_kernel_code_selector(&self) -> SegmentSelector {
        self.kernel_code_selector
    }

    pub fn get_kernel_data_selector(&self) -> SegmentSelector {
        self.kernel_data_selector
    }

    pub fn get_user_code_selector(&self) -> SegmentSelector {
        self.user_code_selector
    }

    pub fn get_user_data_selector(&self) -> SegmentSelector {
        self.user_data_selector
    }

    pub fn get_tss_selector(&self) -> SegmentSelector {
        self.tss_selector
    }
}


/// Accesseur des selecteurs de la gdt courante.
pub fn get_selectors() -> Selectors {
    GDT.1
}


/// S'occupe de déplacer le pointeur de pile lors de double fault.
static GDT: Lazy<(GlobalDescriptorTable, Selectors)> = Lazy::new(|| {
    let mut gdt = GlobalDescriptorTable::new();

    let kernel_code_selector = gdt.append(Descriptor::kernel_code_segment());
    let kernel_data_selector = gdt.append(Descriptor::kernel_data_segment());
    let user_code_selector = gdt.append(Descriptor::user_code_segment());
    let user_data_selector = gdt.append(Descriptor::user_data_segment());

    let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));

    (gdt, Selectors { 
        kernel_code_selector,
        kernel_data_selector,
        user_code_selector,
        user_data_selector,
        tss_selector
    })
});

/// Fonction de chargement de la GDT du kernel..
pub fn init() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, SS, Segment};
   
    print!("Load gdt sector 0 ");
    GDT.0.load();
    print!("(OK)\n");
    unsafe {
        print!("Set kernel code and data segments ");
        CS::set_reg(GDT.1.kernel_code_selector);
        SS::set_reg(GDT.1.kernel_data_selector);
        print!("(OK)\n");
        print!("Load new tss ");
        load_tss(GDT.1.tss_selector);
        print!("(OK)\n");
    }
}
