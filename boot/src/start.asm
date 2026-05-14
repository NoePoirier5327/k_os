[bits 64]
extern _start
global long_mode_start

extern __bss_start
extern __bss_end

long_mode_start:
  ; Nettoyage de la section .bss avant de passer à rust.
  mov al, 0
  mov rdi, __bss_start
  mov rcx, __bss_end
  sub rcx, rdi
  mov al, 0
  rep stosb

  ; On charge 0 dans les registres de segment de données
  mov ax, 0
  mov ss, ax
  mov ds, ax
  mov es, ax
  mov fs, ax
  mov gs, ax

  ; on passe l'adresse de la structure multiboot à rust
  mov rdi, rbx
  call _start
  hlt

section .note.GNU-stack noalloc noexec nowrite progbits
