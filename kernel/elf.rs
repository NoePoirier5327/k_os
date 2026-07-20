use goblin::elf::{Elf, program_header::PT_LOAD};
use x86_64::{VirtAddr, structures::paging::{FrameAllocator, Mapper, Size4KiB, Page, PageTableFlags}};

#[repr(align(2048))]
pub struct AlignedElfBinary<T: ?Sized>(pub T);


/// Charge un executable elf64 pour x86_64 dans la zone mémoire utilisateur.
///
/// # Arguments
/// - elf_bytes : contenu du fichier elf64 à chargé en mémoire.
/// - mapper : mapper mémoire utilisé pour lié la nouvelle frame dans la page qui lui est destiné.
/// - frame_allocator : alloueur de frame mémoire.
///
/// # Return
/// Adresse virtuelle du programme en mémoire.
///
/// # Safety
pub unsafe fn load_elf(
    elf_bytes : &[u8],
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>
) -> VirtAddr {
    let elf = Elf::parse(elf_bytes).expect("Invalid elf64 file.");

    for phdr in &elf.program_headers {
        // On ne s'occupe que de segment chargeable en mémoire.
        if phdr.p_type != PT_LOAD {
            continue;
        }

        let start_vaddr = VirtAddr::new(phdr.p_vaddr);
        let end_vaddr = start_vaddr + phdr.p_memsz;

        // On arrondi aux limites des pages (4 KiB)
        let start_page: Page = Page::containing_address(start_vaddr);
        let end_page: Page = Page::containing_address(end_vaddr - 1u64);

        // Défininition des permissions
        let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
        if phdr.p_flags & goblin::elf::program_header::PF_W != 0 {
            flags |= PageTableFlags::WRITABLE;
        }
        // Note : sur x86_64, la mémoire est exécutable par défaut, 
        // il faudrait activer le flag NO_EXECUTE si PF_X n'est pas présent.

        // On alloue et map chaque page USER_ACCESSIBLE.
        for page in Page::range_inclusive(start_page, end_page) {
            // Si la page est déjà mappée (ex: chevauchement de segments), on ignore
            // (À gérer proprement avec un check mapper.translate_page())
            
            let frame = frame_allocator.allocate_frame().expect("Not enough memory left.");
            mapper.map_to(page, frame, flags, frame_allocator).unwrap().flush();

            // On met TOUTE la page allouée à zéro via le vm_offset.
            // Cela gère automatiquement le padding et la section BSS sans calculs complexes.
            let phys_addr = frame.start_address().as_u64();
            let kernel_vaddr = crate::VIRTUAL_MEMORY_OFFSET + phys_addr;
            core::ptr::write_bytes(kernel_vaddr.as_mut_ptr::<u8>(), 0, 4096);
        }

        // Copie les données de l'ELF en mémoire
        let src_ptr = elf_bytes[phdr.p_offset as usize..].as_ptr();
        let dst_ptr = start_vaddr.as_mut_ptr::<u8>();
        core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, phdr.p_filesz as usize);
    }

    VirtAddr::new(elf.entry)
}
