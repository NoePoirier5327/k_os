[bits 64]
extern _start
global long_mode_start

long_mode_start:
    ; Charger 0 dans les registres de segment de données
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    call _start
    hlt
section .note.GNU-stack noalloc noexec nowrite progbits
