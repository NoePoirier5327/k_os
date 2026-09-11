//! Implémentation d'un TSS pour la gestion des doubles fault.
//! Architecture cible : x86_64
//! Code majoritairement tiré du tutoriel de Philipp Opermann.

use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use spin::{Lazy, Mutex};
use crate::arch::hal::cpu::CpuContext;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// Gère les sélecteurs de segments x86_64.
#[derive(Debug, Clone, Copy)]
pub struct Selector {
    kernel_code_selector: SegmentSelector,
    kernel_data_selector: SegmentSelector,
    user_code_selector: SegmentSelector,
    user_data_selector: SegmentSelector,
    tss_selector: SegmentSelector
}

impl Selector {
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

/// Rpérésente le contexte d'exécution du processeur x86_64
pub struct X86_64CpuContext {
    gdt: GlobalDescriptorTable,
    selectors: Selector,
    tss: Mutex<TaskStateSegment>
}

impl X86_64CpuContext {
    pub fn get_selectors(&self) -> Selector {
        self.selectors
    }
}

pub static X86_64CPU_CONTEXT_INTERFACE: Lazy<X86_64CpuContext> = Lazy::new(|| {
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

    let mut gdt = GlobalDescriptorTable::new();
    let kernel_code_selector = gdt.append(Descriptor::kernel_code_segment());
    let kernel_data_selector = gdt.append(Descriptor::kernel_data_segment());
    let user_code_selector = gdt.append(Descriptor::user_code_segment());
    let user_data_selector = gdt.append(Descriptor::user_data_segment());
    let tss_selector = gdt.append(Descriptor::tss_segment(unsafe {
        core::mem::transmute::<&TaskStateSegment, &'static TaskStateSegment>(&tss)
    }));

    X86_64CpuContext { 
        gdt,
        selectors: Selector {
            kernel_code_selector,
            kernel_data_selector,
            user_code_selector,
            user_data_selector,
            tss_selector
        },
        tss: Mutex::new(tss)
    }
});

impl CpuContext for X86_64CpuContext {
    fn init(&'static self) {
        use x86_64::instructions::tables::load_tss;
        use x86_64::instructions::segmentation::{CS, SS, Segment};

        crate::disp_info!("Loading gdt sector 0.");
        self.gdt.load();

        unsafe {
            crate::disp_info!("Set kernel code and data segments.");
            CS::set_reg(self.selectors.kernel_code_selector);
            SS::set_reg(self.selectors.kernel_data_selector);

            crate::disp_info!("Loading new tss.");
            load_tss(self.selectors.tss_selector);
        }
    }

    fn update_kernel_stack(&self, stack_top: u64) {
        let mut tss = self.tss.lock();
        tss.privilege_stack_table[0] = VirtAddr::new(stack_top);
    }
}
