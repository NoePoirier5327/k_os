ENTRY(start)
OUTPUT_FORMAT(elf64-x86-64)

SECTIONS
{
  . = 1M; /* Chargement à 1 Mo */

  .text : {
    KEEP(*(.multiboot_header)) /* Force le header au début */
    *(.text .text.*)
  }

  .rodata : { *(.rodata .rodata.*) }
  .data   : { *(.data .data.*) }
  .bss    : { *(.bss .bss.*) }
}
