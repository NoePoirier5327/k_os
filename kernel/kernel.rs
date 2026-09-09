//! Contient le singleton du kernel.

use crate::memory::types::PhysAddr;
use crate::arch::init_kernel_memory;
use crate::arch::hal::memory::{FrameAllocator, Mapper};
use crate::arch::without_interrupts;
use alloc::boxed::Box;
use spin::{Once, Mutex};

/// Instance global protégée par un OnceLock.
static KERNEL_INSTANCE: Once<Kernel> = Once::new();

/// Frame allocator du kernel, lui aussi un singleton.
/// Accessible via with_frame_allocator ou on_memory pour avoir le mapper avec.
static FRAME_ALLOCATOR: Once<Mutex<Box<dyn FrameAllocator + Send>>> = Once::new();

/// Mapper du kernel, aussi un singleton.
/// Accessible via with_mapper ou on_memory pour avoir le frame_allocator avec.
static MAPPER: Once<Mutex<Box<dyn Mapper + Send>>> = Once::new();

pub struct Kernel {
    phys_mem_offset: PhysAddr,
}

impl Kernel {
    /// Instancie le singleton du kernel et renvoie un accès 
    ///
    /// # Argument
    /// * `physical_memory_offset`: utile à la manipulation de la mémoire du kernel.
    /// * `multiboot2_info_ptr`: Pointeur vers la table d'informations multiboot2
    ///
    /// # Return
    /// Accès vers la nouvelle instance du kernel (si une instance est déjà en train de tourner,
    /// renvoie son instance à la place).
    pub fn init(physical_memory_offset: u64, multiboot2_info_ptr: u64) -> &'static Kernel {
        let phys_mem_offset = PhysAddr::new(physical_memory_offset);
        let (frame_allocator, mapper) = unsafe { init_kernel_memory(phys_mem_offset, multiboot2_info_ptr) };

        FRAME_ALLOCATOR.call_once(|| {
            Mutex::new(Box::new(frame_allocator))
        });

        MAPPER.call_once(|| {
            Mutex::new(Box::new(mapper))
        });

        KERNEL_INSTANCE.call_once(|| Kernel {
            phys_mem_offset,
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
    pub fn get_phys_mem_offset(&self) -> PhysAddr {
        self.phys_mem_offset
    }

    /// Accesseur de l'instance du frame allocator.
    /// Gère le temps de validité du mutex interne.
    /// Empêche les interruptions durant l'appel.
    pub fn with_frame_allocator<R>(f: impl FnOnce(&mut dyn FrameAllocator) -> R) -> R {
        let frame_allocator = FRAME_ALLOCATOR
            .get()
            .expect("The frame allocator is not initialized.");

        without_interrupts(|| {
            let mut guard = frame_allocator.lock();
            f(&mut **guard)
        })
    }

    /// Accesseur de l'instance du mapper kernel.
    /// Gère les temps de validité du mutex interne.
    /// Empêche les interruptions durant l'appel.
    pub fn with_mapper<R>(f: impl FnOnce(&mut dyn Mapper) -> R) -> R {
        let mapper = MAPPER
            .get()
            .expect("The kernel mapper is not initialized.");

        without_interrupts(|| {
            let mut guard = mapper.lock();
            f(&mut **guard)
        })
    }

    /// Accesseur du frame allocator et du mapper en même temps.
    /// Gère leurs temps de validité.
    /// Empêche les interruptions durant l'appel.
    pub fn with_memory<R>(
        f: impl FnOnce(&mut dyn FrameAllocator, &mut dyn Mapper) -> R
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
            f(&mut **allocator_guard, &mut **mapper_guard)
        })
    }
}
