//! Contient le singleton du kernel.

mod memory;
mod allocator;
pub mod syscalls;
mod user_mode;

use multiboot2::BootInformation;
use multiboot2::BootInformationHeader;
use multiboot2::MemoryMapTag;
use spin::Once;
use spin::Mutex;
use x86_64::registers::control::Cr3;
use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};
use x86_64::structures::paging::OffsetPageTable;
use x86_64::VirtAddr;
use memory::BootInfoFrameAllocator;
use memory::init_mapper;
use x86_64::structures::paging::PhysFrame;

/// Instance global protégée par un OnceLock.
static KERNEL_INSTANCE: Once<Kernel> = Once::new();

/// Frame allocator du kernel, lui aussi un singleton.
/// Accessible via with_frame_allocator
static FRAME_ALLOCATOR: Once<Mutex<BootInfoFrameAllocator>> = Once::new();

pub struct Kernel {
    physical_memory_offset: u64,
    pml4_frame: PhysFrame,
}

impl Kernel {
    /// Instancie le singleton du kernel et renvoie un accès 
    ///
    /// # Argument
    /// * `multiboot2_info_ptr`: Pointeur vers la table d'informations multiboot2
    ///
    /// # Return
    /// Accès vers la nouvelle instance du kernel (si une instance est déjà en train de tourner,
    /// renvoie son instance à la place).
    pub fn init(multiboot2_info_ptr: u64) -> &'static Kernel {
        let physical_memory_offset = 0xFFFF_8000_0000_0000u64;
        super::vga_buffer::init(physical_memory_offset);

        // Vérifications de validitée pour le pointeur multiboot2.
        if multiboot2_info_ptr == 0 {
            panic!("The multiboot2 information pointer is null.");
        }

        if !multiboot2_info_ptr.is_multiple_of(8) {
            crate::disp_warning!("Unaligned multiboot2 information pointer.");
        }

        crate::disp_info!("Initialization of the kernel frame allocator");
        FRAME_ALLOCATOR.call_once(|| 
            Mutex::new(
            {
                let boot_info = unsafe {
                BootInformation::load((multiboot2_info_ptr + physical_memory_offset) as *const BootInformationHeader)
                    .expect("Failed to load multiboot2 boot information.")
                };

                let memory_map_tag = unsafe {
                    let tag = boot_info.memory_map_tag().expect("The memory map tag is required.");
                    &*(tag as *const MemoryMapTag)
                };

                unsafe {
                    BootInfoFrameAllocator::init(memory_map_tag)
                }
            }
            )
        );

        let mut mapper = unsafe { init_mapper(VirtAddr::new(physical_memory_offset)) };

        crate::disp_info!("Initialization of the kernel heap.");
        Kernel::with_frame_allocator(|frame_allocator| {
            allocator::init_heap(&mut mapper, frame_allocator)
                .expect("Failed to initialize kernel's heap.");
        });

        crate::disp_info!("Initialization of the GDT.");
        crate::arch::x86_64::gdt::init();

        crate::disp_info!("Initialization of the IDT");
        crate::arch::x86_64::interrupts::init_idt();

        crate::disp_info!("Initialization of the PICS driver.");
        unsafe { crate::arch::x86_64::interrupts::PICS.lock().initialize() };

        crate::disp_info!("Initialization of the SSE support.");
        unsafe {
            // On active FXSAVE/FXRSTOR et les exceptions SIMD dans CR4
            let mut cr4 = Cr4::read();
            cr4.insert(Cr4Flags::OSFXSR);
            cr4.insert(Cr4Flags::OSXMMEXCPT_ENABLE);
            Cr4::write(cr4);

            // On s'assure que la copie du coprocesseur est désactivée et le monitoring activé dans CR0
            let mut cr0 = Cr0::read();
            cr0.remove(Cr0Flags::EMULATE_COPROCESSOR); // Effacer EM
            cr0.insert(Cr0Flags::MONITOR_COPROCESSOR); // Définir MP
            Cr0::write(cr0);
        }

        crate::disp_info!("Copying kernel pml4 frame.");
        let (pml4_frame, _)= Cr3::read();

        crate::disp_info!("Initialization of the tasker.");
        crate::tasker::Tasker::init();

        crate::disp_info!("Enabling cpu's interruptions.");
        x86_64::instructions::interrupts::enable();

        crate::disp_info!("Enabling syscalls.");
        unsafe {
            let selectors = crate::arch::x86_64::gdt::get_selectors();

            syscalls::init_syscalls(
                selectors.get_kernel_code_selector(),
                selectors.get_kernel_data_selector(),
                selectors.get_user_code_selector(),
                selectors.get_user_data_selector()
            );
        };

        KERNEL_INSTANCE.call_once(|| Kernel {
            physical_memory_offset,
            pml4_frame,
        })
    }

    /// Renvoie un accès vers l'instance du kernel courant.
    ///
    /// #Panic
    /// Si le kernel n'est pas initialisé -> Kernel Panic.
    pub fn on_instance() -> &'static Kernel {
        KERNEL_INSTANCE.get().expect("The kernel is not initialized.")
    }

    /// Accesseur vers l'offset de la mémoire physique.
    pub fn physical_memory_offset(&self) -> u64 {
        self.physical_memory_offset
    }

    /// Créer à la demande un mapper kernel dans le higher half.
    pub fn mapper(&self) -> OffsetPageTable<'static> {
        unsafe { memory::init_mapper(VirtAddr::new(self.physical_memory_offset)) }
    }

    /// Renvoie le cadre physique dans lequel est contenu la pml4 noyau.
    pub fn get_pml4_frame(&self) -> PhysFrame {
        self.pml4_frame
    }

    /// Accesseur de l'instance du frame allocator kernel.
    /// Gère le temps de validité du mutex interne.
    /// Empêche les interruptions durant l'utilisation du frame_allocator.
    pub fn with_frame_allocator<R>(f: impl FnOnce(&mut BootInfoFrameAllocator) -> R) -> R {
        let frame_allocator = FRAME_ALLOCATOR
            .get()
            .expect("The kernel frame allocator is not initialized.");

        x86_64::instructions::interrupts::without_interrupts(|| f(&mut frame_allocator.lock()))
    }
}
