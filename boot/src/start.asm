[bits 64]
extern _start
global long_mode_start

extern __bss_start
extern __bss_end

long_mode_start:
  ; On charge 0 dans les registres de segment de données
  mov ax, 0
  mov ss, ax
  mov ds, ax
  mov es, ax
  mov fs, ax
  mov gs, ax

  ; On s'assure que la partie haute de rdi est propre
  mov edi, edi

  ; On s'assure que le pointeur de pile est placé correctement
  extern stack_top
  mov rsp, stack_top

  ; Nettoyage de la section .bss avant de passer à rust.
  push rdi ; On sauvegarde RDI pendant que stosb travaille
  mov al, 0
  mov rdi, __bss_start
  mov rcx, __bss_end
  sub rcx, rdi
  mov al, 0
  rep stosb

  ; on passe l'adresse de la structure multiboot à rust
  pop rdi
  call _start
  hlt

section .note.GNU-stack noalloc noexec nowrite progbits
