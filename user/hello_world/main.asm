; This a test of user application gestion for the KOs kernel.
; by Noé Poirier
; the 18/06/2026

section .data
hello: db "Hello world!"
hello_len: equ $-hello

section .text
  global _start

_start:
.loop:
  mov rax, 0x00
  mov rdi, hello
  mov rsi, hello_len
  syscall
  jmp .loop
