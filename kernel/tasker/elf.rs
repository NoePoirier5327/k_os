use goblin::elf::{Elf, program_header::PT_LOAD};
use crate::arch::hal::memory::Mapper;
use crate::memory::types::{VirtAddr, Page, PageFlags, InclusivePageRange};
use crate::kernel::Kernel;

#[repr(align(2048))]
pub struct AlignedElfBinary<T: ?Sized>(pub T);


/// Charge un executable elf64 pour x86_64 dans la zone mémoire utilisateur.
///
/// # Arguments
/// - `elf_bytes`: contenu du fichier elf64 à chargé en mémoire.
/// - `user_mapper`: mapper utilisateur chargé de charger l'elf en zone utilisateur.
///
/// # Return
/// Adresse virtuelle du programme en mémoire.
///
/// # Safety
/// - L'executable en paramètre doit être lié pour être dans la zone utilisateur.
/// - L'architecture cible des executables doit être x86_64.
pub unsafe fn load_elf(
    elf_bytes: &[u8],
    user_mapper: &mut dyn Mapper
) -> VirtAddr {
    let elf = Elf::parse(elf_bytes).expect("Invalid elf64 file.");
    let virt_mem_offset = VirtAddr::new(Kernel::on_instance().get_phys_mem_offset().as_u64());

    for phdr in &elf.program_headers {
        if phdr.p_type != PT_LOAD {
            continue;
        }

        let start_vaddr = VirtAddr::new(phdr.p_vaddr);
        let end_vaddr = VirtAddr::new(start_vaddr.as_u64() + phdr.p_memsz);

        let start_page: Page = Page::new(start_vaddr);
        let end_page: Page = Page::new(VirtAddr::new(end_vaddr.as_u64() - 1u64));

        let mut flags = PageFlags::PRESENT | PageFlags::USER_ACCESSIBLE;
        if phdr.p_flags & goblin::elf::program_header::PF_W != 0 {
            flags |= PageFlags::WRITABLE;
        }

        for page in InclusivePageRange::new(start_page, end_page) {
            // On empêche un deadlock en desactivant les interruptions.
            let frame = x86_64::instructions::interrupts::without_interrupts(|| {
                Kernel::with_frame_allocator(|frame_allocator| {
                    let frame = frame_allocator.allocate_frame().expect("Not enough memory left.");
                    user_mapper.map_to(page, frame, flags, frame_allocator).unwrap();

                    frame
                })
            });

            // Adresse virtuelle accessible par le Kernel pour écrire dans la frame.
            let phys_addr = frame.get_start_address().as_u64();
            let kernel_vaddr = VirtAddr::new(virt_mem_offset.as_u64() + phys_addr);
            let page_ptr = kernel_vaddr.as_mut_ptr::<u8>();

            // On initialise la page à zéro.
            core::ptr::write_bytes(page_ptr, 0, 4096);

            // On calcule quelle portion du segment elf doit être copiée dans la page courante.
            let page_start_vaddr = page.get_start_address();
            
            let page_offset_in_segment = if page_start_vaddr > start_vaddr {
                (page_start_vaddr.as_u64() - start_vaddr.as_u64()) as usize
            } else { 0 };

            if page_offset_in_segment < phdr.p_filesz as usize {
                let page_internal_offset = if start_vaddr > page_start_vaddr {
                    (start_vaddr.as_u64() - page_start_vaddr.as_u64()) as usize
                } else { 0 };

                let bytes_to_copy = core::cmp::min(
                    phdr.p_filesz as usize - page_offset_in_segment,
                    4096 - page_internal_offset
                );

                let src_offset = phdr.p_offset as usize + page_offset_in_segment;
                let src_ptr = elf_bytes.as_ptr().add(src_offset);
                let dst_ptr = page_ptr.add(page_internal_offset);

                // On copie directement dans la frame physique du higher half.
                core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, bytes_to_copy);
            }
        }
    }

    VirtAddr::new(elf.entry)
}
