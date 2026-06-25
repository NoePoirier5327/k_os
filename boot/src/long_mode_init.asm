[bits 32]
section .text

; On importe les symbôles ûtiles pour le fichier
extern boot
extern __bss_start
extern __bss_end

global start

; On export les symbôles utiles dans le projet.
global stack_top
global p4_table
global p3_table
global p2_table

; Procédure principale du fichier.
; Passe le CPU en long mode
start:
  cli
  mov esp, stack_top

  mov esi, eax ; On sauvegarde le magic number

  ; Nettoyage de la section .bss
  mov edi, __bss_start
  mov ecx, __bss_end
  sub ecx, edi
  mov al, 0
  rep stosb

  mov eax, esi ; On récupère le magic number

  push ebx ; On sauvegarde le pointeur multiboot2

  call check_multiboot
  call check_cpuid
  call check_long_mode

  call set_up_page_tables
  call enable_paging

  ; On charge la GDT 64 bit
  lgdt [gdt64.pointer]

  ; On affiche BOOT OK en blanc sur fond vert
  mov dword [0xb8000], 0x2f4f2f42
  mov dword [0xb8004], 0x2f542f4f
  mov dword [0xb8008], 0x2f4f2f20
  mov dword [0xb800c], 0x20202f4b
  
  pop edi ; On récupère le pointeur multiboot2

  ; On passe en 64 bit
  jmp gdt64.code:boot

  hlt

; Vérifie la validitée du loader multiboot.
check_multiboot:
  cmp eax, 0x36d76289
  jne .no_multiboot
  ret
.no_multiboot:
  mov al, "0"
  jmp error

; Vérifie si l'identifiant CPU est supporté.
; Code tirée du wiki OSDev
check_cpuid:
  ; Check if CPUID is supported by attempting to flip the ID bit (bit 21)
  ; in the FLAGS register. If we can flip it, CPUID is available.

  ; Copy FLAGS in to EAX via stack
  pushfd
  pop eax

  ; Copy to ECX as well for comparing later on
  mov ecx, eax

  ; Flip the ID bit
  xor eax, 1 << 21

  ; Copy EAX to FLAGS via the stack
  push eax
  popfd

  ; Copy FLAGS back to EAX (with the flipped bit if CPUID is supported)
  pushfd
  pop eax

  ; Restore FLAGS from the old version stored in ECX (i.e. flipping the
  ; ID bit back if it was ever flipped).
  push ecx
  popfd

  ; Compare EAX and ECX. If they are equal then that means the bit
  ; wasn't flipped, and CPUID isn't supported.
  cmp eax, ecx
  je .no_cpuid
  ret
.no_cpuid:
  mov al, "1"
  jmp error

; Vérifie qu'on peut passer en long mode.
; tirée du wiki OSDev
check_long_mode:
  mov eax, 0x80000000    ; Set the A-register to 0x80000000.
  cpuid                  ; CPU identification.
  cmp eax, 0x80000001    ; Compare the A-register with 0x80000001.
  jb .no_long_mode       ; It is less, there is no long mode.
  mov eax, 0x80000001    ; Set the A-register to 0x80000001.
  cpuid                  ; CPU identification.
  test edx, 1 << 29      ; Test if the LM-bit, which is bit 29, is set in the D-register.
  jz .no_long_mode       ; They aren't, there is no long mode.
  ret
.no_long_mode:
  mov esp, stack_top
  mov al, "2"
  jmp error

; Active la pagination processeur.
enable_paging:
  ; On charge la page 4 dans le registre cr3
  mov eax, p4_table
  mov cr3, eax

  ; On active le flag PAE dans le registre cr4
  mov eax, cr4
  or eax, 1 << 5
  mov cr4, eax

  ; On active le bit de long mode dans le EFER MSR
  mov ecx, 0xC0000080
  rdmsr
  or eax, 1 << 8
  wrmsr

  ; On active la pagination dans le registre cr0
  mov eax, cr0
  or eax, 1 << 31
  mov cr0, eax

  ret

; Charge les pages processeur.
set_up_page_tables:
  ; On place la première entrée de la P4 sur la P3
  mov eax, p3_table
  or eax, 0b11 ; present + writable
  mov [p4_table], eax

  ; On place la 256ème entrée de P4 sur la P3
  mov [p4_table + 256 * 8], eax

  ; On place la première entrée de la P3 sur la P2
  mov eax, p2_table
  or eax, 0b11 ; present + writable
  mov [p3_table], eax

  mov ecx, 0 
.map_p2_table:
  ; On map la page courante sur une page à l'adressse 2MiB*ecx
  mov eax, 0x200000  ; 2MiB
  mul ecx            ; adressse de départ
  or eax, 0b10000011 ; present + writable + huge
  mov [p2_table + ecx * 8], eax

  inc ecx
  cmp ecx, 512
  jne .map_p2_table

  ret

; Affiche 'ERROR: ' en blanc sur fond rouge suivit d'un code d'erreur.
; 
; Argument
; * `al` : code d'erreur à afficher.
error:
  mov dword [0xb8000], 0x4f524f45
  mov dword [0xb8004], 0x4f4f4f52
  mov dword [0xb8008], 0x4f3a4f52
  mov dword [0xb800a], 0x4f204f20
  mov byte  [0xb800c], al
  hlt

section .bss
align 4096
p4_table:
  resb 4096
p3_table:
  resb 4096
p2_table:
  resb 4096
stack_bottom:
  resb 65536 ; 64 KiB
stack_top:

section .rodata
gdt64:
  dq 0
.code: equ $ - gdt64 
  dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; segment de code
.pointer:
  dw $ - gdt64 - 1
  dq gdt64

section .note.GNU-stack noalloc noexec nowrite progbits
