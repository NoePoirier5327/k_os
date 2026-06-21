; This a test of user application gestion for the KOs kernel.
; by Noé Poirier
; the 18/06/2026

global _start

section .data
hello: db "Hello world!"
hello_len: equ $-hello

section .text
_start:
  mov rax, 0 ; SYS_DISP
  mov rsi, hello
  mov rdx, hello_len
  syscall

; No sys_exit implemented yet
loop:
  nop
  jmp loop
