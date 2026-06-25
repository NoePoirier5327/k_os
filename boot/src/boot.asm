[bits 64]
extern kernel_start
global boot

boot:
  ; On charge 0 dans les registres de segment de données
  mov ax, 0
  mov ss, ax
  mov ds, ax
  mov es, ax
  mov fs, ax
  mov gs, ax

  ; On s'assure que le pointeur de pile est placé correctement
  extern stack_top
  mov rsp, stack_top

  mov edi, edi

  mov rsi, 0xffff800000000000 ; 2eme argument : physical_memory_offset
  call kernel_start
  hlt

section .note.GNU-stack noalloc noexec nowrite progbits
