[bits 32]
section .text
global start
extern long_mode_start

start:
  mov esp, stack_top

  call check_multiboot
  call check_cpuid
  call check_long_mode

  call setup_page_tables
  call enable_paging

  ; Charger la GDT 64 bits
  lgdt [gdt64.pointer]
  jmp gdt64.code:long_mode_start

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
  ; Pointer la PML4 vers la PDPT
  mov eax, p3_table
  or eax, 0b11 ; present + writable
  mov [p4_table], eax

  ; Pointer la PDPT vers la PD
  mov eax, p2_table
  or eax, 0b11
  mov [p3_table], eax

  ; Mapper chaque entrée de la PD vers une page de 2Mo
  mov ecx, 0
.map_p2_table:
  mov eax, 0x200000  ; 2Mo
  mul ecx
  or eax, 0b10000011 ; present + writable + huge
  mov [p2_table + ecx * 8], eax

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
  ret

error:
  mov dword [0xb8000], 0x4f524f45
  mov byte  [0xb8004], al
  hlt

section .bss
align 4096
p4_table: resb 4096
p3_table: resb 4096
p2_table: resb 4096
stack_bottom: resb 4096
stack_top:

section .rodata
gdt64:
  dq 0 ; null entry
.code: equ $ - gdt64
  dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment
.pointer:
  dw $ - gdt64 - 1
  dq gdt64
