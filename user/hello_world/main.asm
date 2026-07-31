; This a test of user application gestion for the KOs kernel.
; by Noé Poirier
; the 18/06/2026

extern sys_disp

section .data
hello: db "Hello world!"
hello_len: equ $-hello

section .text
  global _start

_start:
  mov rdi, hello
  mov rsi, hello_len
  call sys_disp

; No sys_exit implemented yet
loop:
  nop
  jmp loop
