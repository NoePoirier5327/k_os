ENTRY(start)
OUTPUT_FORMAT(elf64-x86-64)

SECTIONS
{
  /* On commence à 1 Mo, adresse standard pour Multiboot */
  . = 1M;

  /* Section pour le Header Multiboot et le code de démarrage */
  .text :
  {
    __text_start = .;
    /* Le header doit être au tout début pour être trouvé par GRUB */
    KEEP(*(.multiboot_header))
    *(.text .text.*)
    . = ALIGN(4K);
    __text_end = .;
  }

  . = ALIGN(4K);
  .padata :
  {
    *(.padata)
  }

  /* Données en lecture seule (constantes Rust, chaînes de caractères) */
  .rodata :
  {
    __rodata_start = .;
    *(.rodata .rodata.*)
    . = ALIGN(4K);
    __rodata_end = .;
  }

  /* Données initialisées (globales avec une valeur) */
  .data :
  {
    __data_start = .;
    *(.data .data.*)
    . = ALIGN(4K);
    __data_end = .;
  }

  /* Données non-initialisées (IDT, Tables de pages, Stack) */
  .bss :
  {
    __bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    . = ALIGN(4K);
    __bss_end = .;
  }

  /* Symbole global pour marquer la fin du noyau en mémoire physique */
  __kernel_end = .;
}
