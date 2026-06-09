[bits 32]
section .text
global start
extern long_mode_start

start:
  cli
  mov esp, stack_top
  ; On sauvegarde le pointeur donné par grub avant l'activation du long mode
  push ebx

  call check_multiboot
  call check_cpuid
  call check_long_mode

  call setup_page_tables
  call enable_paging

  ; On récupère le pointeur d'info de multiboot donné par grub
  pop edi

  ; Charger la GDT 64 bits
  lgdt [gdt64.pointer]
  jmp gdt64.code:init_64bit

; Trampoline pour s'assurer le passage en mode 64 bits avant le saut vers le kernel
[bits 64]
init_64bit:
  extern long_mode_start
  jmp long_mode_start

[bits 32]
; --- Vérifications de sécurité ---
check_multiboot:
  cmp eax, 0x36d76289
  jne .no_multiboot
  ret
.no_multiboot:
  mov al, "0"
  jmp error

check_cpuid: 
  ret

check_long_mode:
  mov eax, 0x80000000
  cpuid
  cmp eax, 0x80000001
  jb .no_long_mode
  mov eax, 0x80000001
  cpuid
  test edx, 1 << 29
  jz .no_long_mode
  ret
.no_long_mode:
  mov al, "1"
  jmp error

; --- Configuration de la Pagination (Identity Mapping) ---
setup_page_tables:
  ; On fait pointer la 511ème entrée de P4 vers P4 elle-même
  ;mov eax, p4_table
  ;or eax, 0b11 ; present + writable
  ;mov [p4_table + 511 * 8], eax

  ; Pointer la PML4 vers la PDPT
  mov eax, p3_table
  or eax, 0b11 ; present + writable
  mov [p4_table], eax 

  ; On pointe l'entrée 256 de la P4 vers le haut de la p3
  mov eax, p3_higher_table
  or eax, 0b11
  mov [p4_table + 256 * 8], eax   ; Entrée 256 de P4 -> P3

  ; On pointe les tables p3 vers vers notre p2
  mov eax, p2_table
  or eax, 0b11
  mov [p3_table], eax
  mov [p3_higher_table], eax

  ; Mapper chaque entrée de la PD vers une page de 2Mo
  mov ecx, 0
.map_p2_table:
  mov eax, 0x200000
  mul ecx
  or eax, 0b10000011
  mov [p2_table + ecx * 8], eax
  mov [p2_table + ecx * 8 + 4], edx 
  inc ecx
  cmp ecx, 512
  jne .map_p2_table
  ret

enable_paging:
  ; Charger l'adresse de la PML4 dans CR3
  mov eax, p4_table
  mov cr3, eax

  ; Activer PAE (Physical Address Extension) dans CR4
  mov eax, cr4
  or eax, 1 << 5
  mov cr4, eax

  ; Activer le Long Mode dans le MSR EFER
  mov ecx, 0xC0000080
  rdmsr
  or eax, 1 << 8
  wrmsr

  ; Activer la pagination dans CR0
  mov eax, cr0
  or eax, 1 << 31
  mov cr0, eax

  ; Afichage du message de réussite
  mov dword [0xb8000], 0x2f502f41 ; PA
  mov dword [0xb8002], 0x2f472f49 ; GI
  mov dword [0xb8004], 0x2f4e2f47 ; NG
  mov dword [0xb8006], 0x2f202f45 ;  E
  mov dword [0xb8008], 0x2f4e2f41 ; NA
  mov dword [0xb800a], 0x2f422f4c ; BL
  mov dword [0xb800c], 0x2f452f44 ; ED

  ; Retour à l'appelant
  ret

error:
  mov dword [0xb8000], 0x4f524f45
  mov byte  [0xb8004], al
  hlt

; Section de paging contenant notemment la IDT
section .padata
align 4096
global p4_table
global stack_top

p4_table: times 4096 db 0
p3_table: times 4096 db 0 ; Pour la partie basse de la table p3
p3_higher_table: times 4096 db 0 ; Pour la partie haute
p2_table: times 4096 db 0
stack_bottom: times 16384 db 0
stack_top:

; Section en lecture seule
section .rodata
align 8
gdt64:
  dq 0 ; null entry
.code: equ $ - gdt64
  dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment (64-bit, present, code)
.pointer:
  dw $ - gdt64 - 1
  dq gdt64 

section .note.GNU-stack noalloc noexec nowrite progbits
