//! Module de gestion de la mémoire de l'architecture x86_64.

use alloc::vec::Vec;
use spin::Lazy;
use x86_64::structures::paging::{
    FrameAllocator as X86_64FrameAllocatorTrait,
    Mapper as X86_64MapperTrait,
    Page as X86_64Page,
    PageTable as X86_64PageTable,
    PageTableFlags as X86_64PageFlags,
    PhysFrame as X86_64PhysFrame,
    Size4KiB,
    Translate,
    OffsetPageTable,
};
use x86_64::registers::model_specific::{
    Efer,
    EferFlags
};
use x86_64::registers::control::{
    Cr0,
    Cr0Flags,
    Cr3,
    Cr4,
    Cr4Flags
};
use x86_64::{
    VirtAddr as X86_64VirtAddr,
    PhysAddr as X86_64PhysAddr
};

use multiboot2::{BootInformation, BootInformationHeader, MemoryAreaType, MemoryMapTag};
use crate::arch::hal::memory::{FrameAllocator, Mapper};
use crate::memory::types::{MemoryAllocationError, Page, PageFlags, PhysAddr, PhysFrame, VirtAddr};
use crate::kernel::Kernel;

// Adresses de début et fin du kernel.
extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
}

/// Récupère, au démarrage du kernel, sa pml4 initial.
/// Fonctionne car appelé dans init_kernel_memory qui est appelé au démarrage du kernel.
const INITIAL_KERNEL_PML4: Lazy<X86_64PhysFrame> = Lazy::new(|| {
    let (pml4, _) = Cr3::read();
    pml4
});

/// Initialise les composantes mémoire spécifique à l'architecture x86_64
/// et renvoie un couple (frame_allocator/mapper) dédié au kernel.
///
/// # Safety
/// Ne doit être appelé qu'au démarrage du kernel.
pub unsafe fn init_kernel_memory(
    phys_mem_offset: PhysAddr,
    boot_info_ptr: u64
) -> (X86_64FrameAllocator, X86_64Mapper<'static>) {
    crate::disp_info!("Enabling no-execute (NX) bit support.");
    unsafe {
        let mut efer = Efer::read();
        efer.insert(EferFlags::NO_EXECUTE_ENABLE);
        Efer::write(efer);
    }

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

    crate::disp_info!("Loading multiboot2 information pointer.");
    let boot_info = BootInformation::load((boot_info_ptr + phys_mem_offset.as_u64()) as *const BootInformationHeader)
        .expect("Failed to load multiboot2 information pointer.");

    crate::disp_info!("Extracting the memory map tag from the multiboot2 information pointer.");
    let memory_map_tag = {
        let tag = boot_info.memory_map_tag().expect("The memory map tag is required.");
        &*(tag as *const MemoryMapTag)
    };

    crate::disp_info!("Instanciating the x86_64 frame allocator.");
    let frame_allocator = X86_64FrameAllocator::new(memory_map_tag);

    crate::disp_info!("Instanciating the x86_64 kernel mapper.");
    let kernel_mapper = X86_64Mapper::new(phys_mem_offset, *INITIAL_KERNEL_PML4);

    (
        frame_allocator,
        kernel_mapper
    )
}

/// Instancie un nouveau mapper utilisateur indépendant.
pub unsafe fn new_user_mapper() -> X86_64Mapper<'static> {
    let phys_mem_offset = Kernel::on_instance().get_phys_mem_offset();
    let user_pml4 = new_user_pml4(phys_mem_offset);
    X86_64Mapper::new(phys_mem_offset, user_pml4)
}

/// Créer une nouvelle pml4 utilisateur à partir de la pml4 kernel.
fn new_user_pml4(phys_mem_offset: PhysAddr) -> X86_64PhysFrame {
    // On accède aux valeurs kernel
    let initial_pml4 = *INITIAL_KERNEL_PML4;

    // On alloue la nouvelle pml4
    let new_pml4_frame = Kernel::with_frame_allocator(|frame_allocator| {
        let frame = frame_allocator
            .allocate_frame()
            .expect("No more memory left to allocate user's pml4.");
        X86_64PhysFrame::containing_address(X86_64PhysAddr::new(frame.get_start_address().as_u64()))
    });

    let new_pml4: &mut X86_64PageTable = unsafe {
        let phys_addr = new_pml4_frame.start_address().as_u64();
        let virt_addr = X86_64VirtAddr::new(phys_addr + phys_mem_offset.as_u64());
        &mut *virt_addr.as_mut_ptr::<X86_64PageTable>()
    };

    // On la néttoie
    new_pml4.zero();

    // On récupère la pml4 kernel.
    let kernel_pml4 = unsafe {
        let phys_addr = initial_pml4.start_address().as_u64();
        let virt_addr = X86_64VirtAddr::new(phys_addr + phys_mem_offset.as_u64());
        &*virt_addr.as_ptr::<X86_64PageTable>()
    };

    // On copie la partie haute de la table kernel dans la table utilisateur.
    for i in 256..512 {
        new_pml4[i] = kernel_pml4[i].clone();
    }

    // La partie basse sera peuplée lors du chargement du binaire
    // et de la pile utilisateur.

    new_pml4_frame
}

/// Structure d'un alloueur mémoire simple pour l'architecture x86_64.
pub struct X86_64FrameAllocator {
    memory_map : &'static MemoryMapTag,
    current_region_index: usize,
    current_address : u64,
    recycle_bin: Vec<X86_64PhysFrame>
}

impl X86_64FrameAllocator {
    /// Fonction de création d'un alloueur à partir d'une carte de la mémoire de multiboot2.
    /// 
    /// # Argument
    /// * `memory_map` : carte de la mémoire obtenue via multiboot2.
    ///
    /// # Return
    /// Instance de l'alloueur.
    ///
    /// # Safety
    /// L'appelant doit garantir que la carte de la mémoire est valide.
    pub unsafe fn new(memory_map: &'static MemoryMapTag) -> Self {
        let memory_areas = memory_map.memory_areas();
        let first_address = memory_areas.first().map(|address| address.start_address()).unwrap_or(0);

        Self {
            memory_map,
            current_region_index: 0,
            current_address: first_address,
            recycle_bin: Vec::new()
        }
    }

    /// Calcul et renvoie la n-ième frame utilisable
    fn calculate_usable_frames(&mut self) -> Option<X86_64PhysFrame> {
        // On recycle si possible les anciennes frames désallouées.
        if let Some(frame) = self.recycle_bin.pop() {
            return Some(frame)
        }

        // On récupère un accès vers les pages du kernel
        let kernel_start = core::ptr::addr_of!(__kernel_start) as u64;
        let kernel_end = core::ptr::addr_of!(__kernel_end) as u64;

        let mut to_return = None;

        let memory_areas = self.memory_map.memory_areas();
        while self.current_region_index < memory_areas.len() && to_return.is_none() {
            // On récupère la région courante.
            let current_region = &memory_areas[self.current_region_index];

            // Si elle n'est pas accessible, on passe à la suivante.
            if current_region.typ() != MemoryAreaType::Available {
                self.current_region_index += 1;

                // Si on peut encore exécuter une nouvelle boucle,
                // alors on met à jour l'adresse courante.
                if self.current_region_index < memory_areas.len() {
                    self.current_address = memory_areas[self.current_region_index].start_address();
                }

                // On force le passage à la nouvelle itération de la boucle.
                continue;
            }

            // On aligne l'adresse courante sur 4096 octets.
            let current_address = (self.current_address + 4095) & !4095;

            // Si l'adresse courante dépasse la fin de la région courante,
            // on passe à la suivante
            if current_address + 4096 > current_region.end_address() {
                self.current_region_index += 1;

                // Si on peut encore exécuter une nouvelle boucle,
                // alors on met à jour l'adresse courante.
                if self.current_region_index < memory_areas.len() {
                    self.current_address = memory_areas[self.current_region_index].start_address();
                }

                // On force le passage à la nouvelle itération de la boucle.
                continue;
            }

            // On fait avancer l'adresse courante pour le prochain appel.
            self.current_address = current_address + 4096;

            // On ignore l'adresse NULL et les adresses kernel.
            if current_address == 0 || (current_address + 4096 > kernel_start && current_address < kernel_end) {
                continue;
            }

            // Enfin, toutes les étapes sont passées, on peut renvoyer un cadre valide.
            to_return = Some(X86_64PhysFrame::containing_address(X86_64PhysAddr::new(current_address)));
        }

        to_return
    }
}

impl FrameAllocator for X86_64FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.calculate_usable_frames().map(|frame| {
            PhysFrame::new(
                PhysAddr::new(
                    frame.start_address().as_u64()
                )
            )
        })
    }

    fn deallocate_frame(&mut self, frame: PhysFrame) {
        // On repasse au format de données spécifique pour x86_64
        let x86_64_frame = X86_64PhysFrame::containing_address(
            X86_64PhysAddr::new(
                frame.get_start_address().as_u64()
            )
        );

        // Puis, on désalloue le cadre.
        self.recycle_bin.push(x86_64_frame);
    }
}

/// Convertit un frame allocator générique en frame allocator spécifique à x86_64.
struct FrameAllocatorConverter<'a> (&'a mut dyn FrameAllocator);

unsafe impl<'a> X86_64FrameAllocatorTrait<Size4KiB> for FrameAllocatorConverter<'a> {
    fn allocate_frame(&mut self) -> Option<X86_64PhysFrame<Size4KiB>> {
        self.0.allocate_frame().map(|frame| {
            X86_64PhysFrame::containing_address(X86_64PhysAddr::new(frame.get_start_address().as_u64()))
        })
    }
}

/// Structure d'un mapper x86_64 compatible avec le frame allocator défini au dessus.
pub struct X86_64Mapper<'a> {
    pml4_frame: X86_64PhysFrame,
    x86_64_mapper: OffsetPageTable<'a>
}

impl<'a> X86_64Mapper<'a> {
    /// Instancie et renvoie un mapper x86_64 créé à partir de l'offset en paramètre et de la pml4
    /// correspondant à son utilisation.
    ///
    /// # Safety
    /// L'offset doit être valide,
    /// La pml4 doit correspondre à celle du contexte cible.
    pub unsafe fn new(phys_mem_offset: PhysAddr, pml4_frame: X86_64PhysFrame) -> Self {
        // On récupère un accès mutable au pointeur interne de la pml4 en paramètre.
        let virt_mem_offset = X86_64VirtAddr::new(phys_mem_offset.as_u64());
        let phys_frame = pml4_frame.start_address();
        let virt_frame = virt_mem_offset + phys_frame.as_u64();
        let page_table_ptr: *mut X86_64PageTable = virt_frame.as_mut_ptr();

        // On instancie le OffsetPageTable interne.
        let x86_64_mapper = OffsetPageTable::new(&mut * page_table_ptr, virt_mem_offset);
        
        Self {
            pml4_frame,
            x86_64_mapper
        }
    }
}

impl<'a> Mapper for X86_64Mapper<'a> {
    unsafe fn map_to(
        &mut self,
        page: Page,
        frame: PhysFrame,
        flags: PageFlags,
        frame_allocator: &mut dyn FrameAllocator
    ) -> Result<(), MemoryAllocationError> {
        // On récupère les accès à la mémoire sous le format x86_64 de la crate du même nom.
        let x86_64_page: X86_64Page<Size4KiB> = X86_64Page::containing_address(X86_64VirtAddr::new(page.get_start_address().as_u64()));
        let x86_64_frame = X86_64PhysFrame::containing_address(X86_64PhysAddr::new(frame.get_start_address().as_u64()));

        // On récupère les flags sous le format x86_64.
        let mut x86_64_page_flags = X86_64PageFlags::empty();
        if flags.contains(PageFlags::PRESENT)           { x86_64_page_flags |= X86_64PageFlags::PRESENT;        }
        if flags.contains(PageFlags::WRITABLE)          { x86_64_page_flags |= X86_64PageFlags::WRITABLE;       }
        if flags.contains(PageFlags::USER_ACCESSIBLE)   { x86_64_page_flags |= X86_64PageFlags::USER_ACCESSIBLE;}
        if flags.contains(PageFlags::NO_EXEC)           { x86_64_page_flags |= X86_64PageFlags::NO_EXECUTE;     }

        // On convertit l'allocateur générique en paramètre en allocateur spécifique à l'architecture x86_64.
        let mut x86_64_frame_allocator = FrameAllocatorConverter(frame_allocator);

        // On appelle l'offset page table interne pour le mappage
        self.x86_64_mapper
            .map_to(x86_64_page, x86_64_frame, x86_64_page_flags, &mut x86_64_frame_allocator)
            .map_err(|_| MemoryAllocationError::MappingFailed)?
            .flush();
        
        Ok(())
    }

    unsafe fn unmap(
        &mut self,
        page: Page,
    ) -> Result<PhysFrame, MemoryAllocationError> {
        // On convertit la page générique en paramètre en page x86_64.
        let x86_64_page: X86_64Page<Size4KiB> = X86_64Page::containing_address(X86_64VirtAddr::new(page.get_start_address().as_u64()));

        // On appelle le mapper interne démapper la page en paramètre.
        let (phys_frame, mapper_flush) = self.x86_64_mapper.unmap(x86_64_page)
            .map_err(|_| MemoryAllocationError::UnmappingFailed)?;

        // Si aucune erreur, on le signale.
        mapper_flush.flush();

        // Puis, on renvoie la frame démappée sous son format générique.
        Ok(
            PhysFrame::new(
                PhysAddr::new(phys_frame.start_address().as_u64())
            )
        )
    }

    fn translate(
        &self,
        vaddr: VirtAddr
    ) -> Option<PhysAddr> {
        self.x86_64_mapper.translate_addr(X86_64VirtAddr::new(vaddr.as_u64()))
            .map(|p| PhysAddr::new(p.as_u64()))
    }

    unsafe fn set_as_current(&self) {
        let (_, flags) = Cr3::read();
        Cr3::write(self.pml4_frame, flags);
    }
}
