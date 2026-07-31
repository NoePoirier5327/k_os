; Librairie d'abstraction pour les syscalls du kernel rust kos.

section .text
  global sys_disp
  global sys_dispcolor

; Interface pour sys_disp. 
; Affiche la chaîne de caractère en paramètre.
; 
; Argument
;   RDI : pointeur vers la chaîne de caractère à afficher.
;   RSI : taille de la chaîne à afficher.
;
; Registre modifié et non restauré
;   RAX
sys_disp:
  mov rax, 0x0
  syscall
  ret

; Interface pour sys_dispcolor.
; Change la couleur d'affichage des caractères.
;
; Arguments
;   RDI : couleur des caractères à afficher.
;   RSI : couleur de fond derrière les caractères.
;
; Registre modifié et non restauré
;   RAX
;
; Préconditions
;   RDI et RSI doivent être compris entre 0 et 15 compris.
sys_dispcolor:
  mov rax, 0x1
  syscall
  ret
