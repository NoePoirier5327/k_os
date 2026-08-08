; This is a program to test the kos kernel reactions to unknown syscalls.
; by Noé Poirier
; the 08/08/2026

section .text
  global _start

_start:
  mov rax, 0xF5 ; unknown syscall.
  syscall       ; should fail and display a warning message

; No sys_exit implemented yet.
.loop:
  nop
  jmp .loop
