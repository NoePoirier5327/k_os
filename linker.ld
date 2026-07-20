ENTRY(start)
OUTPUT_FORMAT(elf64-x86-64)

SECTIONS
{
    /* On commence à 1 Mo, adresse standard pour Multiboot */
    . = 1M;

    .boot :
    {
        /* Le header Multiboot doit être dans les premiers octets physiques */
        KEEP(*(.multiboot_header))
        *(.boottext)
        *(.bootdata)
        *(.bootbss)
    }

    /* On passe au higher-half */
    /* On définit l'offset pour le reste du noyau (MCMODEL=KERNEL) */
    . += 0xFFFFFFFF80000000;

    __kernel_start = .;

    /* Code du noyau Rust */
    .text : AT(ADDR(.text) - 0xFFFFFFFF80000000)
    {
        __text_start = .;
        *(.text .text.*)
        . = ALIGN(4K);
        __text_end = .;
    }

    /* Données en lecture seule */
    .rodata : AT(ADDR(.rodata) - 0xFFFFFFFF80000000)
    {
        __rodata_start = .;
        *(.rodata .rodata.*)
        . = ALIGN(4K);
        __rodata_end = .;
    }

    /* Données initialisées */
    .data : AT(ADDR(.data) - 0xFFFFFFFF80000000)
    {
        __data_start = .;
        *(.data .data.*)
        . = ALIGN(4K);
        __data_end = .;
    }

    /* Données non-initialisées et pile */
    .bss : AT(ADDR(.bss) - 0xFFFFFFFF80000000)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4K);
        __bss_end = .;
    }

    __kernel_end = .;
}
