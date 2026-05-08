ENTRY(kernel_entry)
OUTPUT_FORMAT(binary)

SECTIONS
{
    . = 0x1000; /* Adresse de début du kernel*/

    .text : {
        *(.text)
    }

    .data : {
        *(.data)
    }

    .bss : {
        *(.bss)
    }
}
