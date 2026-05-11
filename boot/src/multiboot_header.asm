section .multiboot_header
header_start:
    dd 0xe85250d6                ; Nombre magique (Multiboot2)
    dd 0                         ; Architecture 0 (i386 mode protégé)
    dd header_end - header_start ; Longueur du header
    ; Checksum
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))

    ; Tags optionnels ici (ex: framebuffer, entry address)
    
    dw 0    ; Type
    dw 0    ; Flags
    dd 8    ; Taille
header_end:
section .note.GNU-stack noalloc noexec nowrite progbits
