; Librairie mathématique pour le kernel rust kos.

section .data
  half: dq 0.5
  one: dq 1.0

section .text
  global sqrt
  global pow


; Fonction de calcul de la racine carré d'un réel en double précision.
; Implémentation de l'algorithme de Héron (cas particulier de l'aproximation de Newton).
;
; Arguments
;   - XMM0 : réel dont on veut la racine. (double précision)
;   - RDI  : nombre d'itération de l'algorithme de Héron.
; 
; Registre de retour -> XMM0
;
; Registres modifiées
;   XMM1, XMM2
sqrt:
  movsd xmm1, xmm0 ; u0 = a
  dec rdi ; Index de boucle

.loop:
  ; u_n+1 = 0.5 * (u_n + (a / u_n))
  movapd xmm2, xmm0    ; xmm2 = a
  divsd  xmm2, xmm1    ; xmm2 = a / u_n
  addsd  xmm2, xmm1    ; xmm2 = u_n + (a / u_n)
  mulsd  xmm2, [half]  ; xmm2 = 0.5 * (a / u_n)
  movsd  xmm1, xmm2    ; u_n = u_n+1

  dec rdi
  jnz .loop

.end:
  movsd xmm0, xmm1 ; return u_n
  ret

; Fonction de calcul de puissance.
; Implémentation naïve par multiplication successive.
;
; Arguments
;   - XMM0 : Element dont on veut calculer la puissance. (double précision)
;   - RDI  : Puissance n, entier positif ou nul.
;
; Registre de retour -> XMM0
;
; Registres modifiées
;   XMM1
pow:
  cmp rdi, 0
  je .end_no_loop

  movsd xmm1, xmm0
  dec rdi

.loop:
  mulsd xmm0, xmm1
  dec rdi
  jnz .loop ; Si rdi != 0, alors on boucle.

  ret ; return xmm0

; Si n = 0 alors x^n = 1
.end_no_loop:
  movsd xmm0, [one]
  ret ; return xmm0
