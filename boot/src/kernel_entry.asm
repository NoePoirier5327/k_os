section .text

[bits 32]
[extern _start]

global kernel_entry
kernel_entry:
  call _start
  jmp $
