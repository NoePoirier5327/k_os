; Librairie mathématique pour le kernel rust kos.

section .data
  half: dq 0.5

section .text
  global sqrt

; Fonction de calcul de la racine carré d'un réel en double précision.
; Implémentation de l'algorithme de Héron (cas particulier de l'aproximation de Newton).
;
; Arguments
;   - XMM0: réel dont on veut la racine. (double précision)
;   - RDI: nombre d'itération de l'algorithme de Héron.
; 
; Registre de retour -> XMM0
sqrt:
  movsd xmm1, xmm0 ; u0 = a
  dec rdi ; Index de boucle

.loop:
  ; u_n+1 = 0.5 * (u_n + (a / u_n))
  movapd xmm2, xmm0   ; xmm2 = a
  divsd xmm2, xmm1    ; xmm2 = a / u_n
  addsd xmm2, xmm1    ; xmm2 = u_n + (a / u_n)
  mulsd xmm2, [half]  ; xmm2 = 0.5 * (a / u_n)
  movsd xmm1, xmm2    ; u_n = u_n+1

  dec rdi
  jnz .loop

  movsd xmm0, xmm1 ; return u_n
  ret

