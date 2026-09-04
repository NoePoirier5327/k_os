[bits 64]
section .boottext

extern kernel_start
extern __bss_start
extern __bss_end
global boot

boot:
  ; On charge 0 dans les registres de segment de données
  mov ax, 0
  mov ss, ax
  mov ds, ax
  mov es, ax
  mov fs, ax
  mov gs, ax

  ; On sauvegarde le pointeur multiboot2.
  mov edi, edi
  mov r8, rdi

  ; On nettoie la section bss
  mov rdi, __bss_start
  mov rcx, __bss_end
  sub rcx, rdi
  xor al, al
  rep stosb

  ; On s'assure que le pointeur de pile est placé correctement
  extern stack_top
  mov rsp, stack_top
  add rsp, rsi ; On place la pile dans le higher half

  mov rdi, r8 ; On récupère le pointeur multiboot2.

  mov rax, kernel_start ; Pour accéder au début du noyau en higher-half
  call rax
  hlt

section .note.GNU-stack noalloc noexec nowrite progbits
