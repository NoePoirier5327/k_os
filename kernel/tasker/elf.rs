use goblin::elf::{Elf, program_header::PT_LOAD};
use x86_64::{VirtAddr, structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags}};
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
    user_mapper: &mut OffsetPageTable<'static>
) -> VirtAddr {
    let elf = Elf::parse(elf_bytes).expect("Invalid elf64 file.");
    let virt_mem_offset = VirtAddr::new(Kernel::on_instance().physical_memory_offset());

    for phdr in &elf.program_headers {
        if phdr.p_type != PT_LOAD {
            continue;
        }

        let start_vaddr = VirtAddr::new(phdr.p_vaddr);
        let end_vaddr = start_vaddr + phdr.p_memsz;

        let start_page: Page = Page::containing_address(start_vaddr);
        let end_page: Page = Page::containing_address(end_vaddr - 1u64);

        let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
        if phdr.p_flags & goblin::elf::program_header::PF_W != 0 {
            flags |= PageTableFlags::WRITABLE;
        }

        for page in Page::range_inclusive(start_page, end_page) {
            // On empêche un deadlock en desactivant les interruptions.
            let frame = x86_64::instructions::interrupts::without_interrupts(|| {
                Kernel::with_frame_allocator(|frame_allocator| {
                    let frame = frame_allocator.allocate_frame().expect("Not enough memory left.");
                    user_mapper.map_to(page, frame, flags, frame_allocator).unwrap().flush();

                    frame
                })
            });

            // Adresse virtuelle accessible par le Kernel pour écrire dans la frame.
            let phys_addr = frame.start_address().as_u64();
            let kernel_vaddr = virt_mem_offset + phys_addr;
            let page_ptr = kernel_vaddr.as_mut_ptr::<u8>();

            // On initialise la page à zéro.
            core::ptr::write_bytes(page_ptr, 0, 4096);

            // On calcule quelle portion du segment elf doit être copiée dans la page courante.
            let page_start_vaddr = page.start_address();
            
            let page_offset_in_segment = if page_start_vaddr > start_vaddr {
                (page_start_vaddr - start_vaddr) as usize
            } else { 0 };

            if page_offset_in_segment < phdr.p_filesz as usize {
                let page_internal_offset = if start_vaddr > page_start_vaddr {
                    (start_vaddr - page_start_vaddr) as usize
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
