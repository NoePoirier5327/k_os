//! Contient le singleton du kernel.

use crate::memory::stack::KernelStackAllocator;
use crate::memory::types::{PhysAddr, VirtAddr};
use crate::arch::hal::memory::{FrameAllocator, Mapper, init_kernel_memory};
use crate::arch::without_interrupts;
use spin::{Once, Mutex};

/// Instance global protégée par un OnceLock.
static KERNEL_INSTANCE: Once<Mutex<Kernel>> = Once::new();

/// Frame allocator du kernel, lui aussi un singleton.
/// Accessible via with_frame_allocator ou on_memory pour avoir le mapper avec.
static FRAME_ALLOCATOR: Once<Mutex<FrameAllocator>> = Once::new();

/// Mapper du kernel, aussi un singleton.
/// Accessible via with_mapper ou on_memory pour avoir le frame_allocator avec.
static MAPPER: Once<Mutex<Mapper>> = Once::new();

pub struct Kernel {
    phys_mem_offset: PhysAddr,

    /// Alloueur de haut de pile kernel.
    stack_allocator: KernelStackAllocator
}

impl Kernel {
    /// Instancie le singleton du kernel et renvoie un accès 
    ///
    /// # Argument
    /// * `physical_memory_offset`: utile à la manipulation de la mémoire du kernel.
    /// * `multiboot2_info_ptr`: Pointeur vers la table d'informations multiboot2.
    pub fn init(physical_memory_offset: u64, multiboot2_info_ptr: u64) {
        let phys_mem_offset = PhysAddr::new(physical_memory_offset);
        let (frame_allocator, mapper) = unsafe { init_kernel_memory(phys_mem_offset, multiboot2_info_ptr) };

        FRAME_ALLOCATOR.call_once(|| {
            Mutex::new(frame_allocator)
        });

        MAPPER.call_once(|| {
            Mutex::new(mapper)
        });

        KERNEL_INSTANCE.call_once(||
            Mutex::new(
                Kernel {
                    phys_mem_offset,
                stack_allocator: KernelStackAllocator::new()
                }
            )
        );
    }

    /// Renvoie un accès en lecture vers l'instance du kernel courant.
    /// Gère le temps de vie et d'accès du mutex interne.
    ///
    /// # Panic
    /// Si le kernel n'est pas initialisé -> Kernel Panic.
    // NOTE Ne désactive pas les interruptions à l'appel.
    pub fn on_instance<R>(f: impl FnOnce(&Kernel) -> R) -> R {
        let kernel = KERNEL_INSTANCE.get().expect("The kernel is not initialized.");
        let guard = kernel.lock();
        f(&guard)
    }

    /// Renvoie un accès mutable vers l'instance du kernel courant.
    /// Gère le temps de vie et l'accès au mutex interne.
    ///
    /// # Panic
    /// Si le kernel n'est pas initialisé -> Kernel panic.
    // NOTE Ne désactive pas les interruptions à l'appel.
    pub fn on_instance_mut<R>(f: impl FnOnce(&mut Kernel) -> R) -> R {
        let kernel = KERNEL_INSTANCE.get().expect("The kernel is not initialized.");
        let mut guard = kernel.lock();
        f(&mut guard)
    }

    /// Accesseur vers l'offset de la mémoire physique.
    pub fn get_phys_mem_offset(&self) -> PhysAddr {
        self.phys_mem_offset
    }

    /// Alloue une nouvelle pile noyau dans les pages dédiées.
    pub fn allocate_stack_top(&mut self) -> VirtAddr {
        self.stack_allocator.allocate_top()
    }

    /// Désalloue le haut de pile noyau en paramètre.
    pub fn deallocate_stack_top(&mut self, stack_top: VirtAddr) {
        self.stack_allocator.deallocate_top(stack_top);
    }

    /// Accesseur de l'instance du frame allocator.
    /// Gère le temps de validité du mutex interne.
    /// Empêche les interruptions durant l'appel.
    pub fn with_frame_allocator<R>(f: impl FnOnce(&mut FrameAllocator) -> R) -> R {
        let frame_allocator = FRAME_ALLOCATOR
            .get()
            .expect("The frame allocator is not initialized.");

        without_interrupts(|| {
            let mut guard = frame_allocator.lock();
            f(&mut guard)
        })
    }

    /// Accesseur de l'instance du mapper kernel.
    /// Gère les temps de validité du mutex interne.
    /// Empêche les interruptions durant l'appel.
    pub fn with_mapper<R>(f: impl FnOnce(&mut Mapper) -> R) -> R {
        let mapper = MAPPER
            .get()
            .expect("The kernel mapper is not initialized.");

        without_interrupts(|| {
            let mut guard = mapper.lock();
            f(&mut guard)
        })
    }

    /// Accesseur du frame allocator et du mapper en même temps.
    /// Gère leurs temps de validité.
    /// Empêche les interruptions durant l'appel.
    pub fn with_memory<R>(
        f: impl FnOnce(&mut FrameAllocator, &mut Mapper) -> R
    ) -> R {
        let frame_allocator = FRAME_ALLOCATOR
            .get()
            .expect("The frame allocator is not initialized.");

        let mapper = MAPPER
            .get()
            .expect("The kernel mapper is not initialized.");

        without_interrupts(|| {
            let mut allocator_guard = frame_allocator.lock();
            let mut mapper_guard = mapper.lock();
            f(&mut allocator_guard, &mut mapper_guard)
        })
    }
}
