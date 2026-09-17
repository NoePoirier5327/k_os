; These macros where written by FELSNER Felipe in 2026.
; This is a list of Common Assembly Syscall Macros for the 
; NASM assembler.

; These macros all target a x86_64 calling convention this 
; should not be used in other architectures.


; For future revitions, here is an example on how all 
; these macro should be defined:
/*
; NAME
; description
%macro foobar 6
	mov rax, 0  ; Syscall foobar 
	mov rdi, %1 ; Param 1 (type)
	mov rsi, %2 ; Param 2 (type)
	mov rdx, %3 ; Param 3 (type)
	mov r10, %4 ; Param 4 (type)
	mov r8,  %5 ; Param 5 (type)
	mov r9,  %6 ; Param 6 (type)
	syscall     ; The return value is stored in the rax register
%endmacro
; Return description
*/

; Types are the same used by the GNU GCC C++ compiler

; SYS_DISP
; Displays the message pointed by the parameter on the screen
%macro sys_disp 2
	mov rax, 0x00  ; syscall sys_disp
	mov rdi, %1    ; msg     (char*)
	mov rsi, %2    ; msg_len (size_t)
	syscall        ; the return value is stored in the rax register
%endmacro
; On sucess a positive number is returned, otherwise a negative error code will be returned

;SYS_DISPCOLOR
; Changes the display color for the VGA buffer.
%macro sys_dispcolor 2
	mov rax, 0x01  ; syscall sys_dispcolor
	mov rdi, %1    ; ft_color (char)
	mov rsi, %2    ; bg_color (char)
	syscall        ; the return value is stored in the rax register
%endmacro
; On sucess a positive number is returned, otherwise a negative error code will be returned

;SYS_PALLOC
; allocates a user accessible 4kB page.
%macro sys_palloc 0
	mov rax, 0x02  ; syscall sys_palloc
	syscall        
%endmacro
; [no documentation is available, behaviour unknown]

;SYS_KEYREAD
; returns the last character typed by user.
%macro sys_keyread 0
	mov rax, 0x03  ; syscall sys_keyread 
	syscall        ; the return value is stored in the rax register 
%endmacro
; On sucess returns key, otherwise a negative error code will be returned.

;SYS_FOWN
; Changes owner of file given as parameter.
%macro sys_fown 3
	mov rax, 0x04
	mov rdi, %1   ; owner          (?)
	mov rsi, %2   ; file_path      (char *) 
	mov rdx, %2   ; file_path_size (size_t)
	syscall      
%endmacro
; [no documentation is available, behaviour unknown]

;POSIX_EXECVE
; executes an elf64 application on the caller's process
%macro posix_execve 3
	mov rax, 0x05
	mov rdi, %1   ; file_path (c_str)
	mov rsi, %2   ; argv      (c_str[])
	mov rdx, %2   ; env_var   (c_str[])
	syscall
%endmacro
; [no documentation is available, behaviour unknown]

;SYS_SHUTDOWN
; Kills the kernel as well as turning off the machine.
%macro sys_shutdown 0
	mov rax, 0x06 ; syscall sys_shutdown
	syscall
%endmacro
; [no documentation is available, behaviour unknown]

;SYS_PKILL
; NOT EMPLEMENTED DUE TO THE DESCRIPTION BEING VERY DIFFICULT TO 
; UNDERSTAND, THIS SHOULD BE PATCHED IN THE FUTURE. IF NOT PATCHED YET
; CHECK THE DOCUMENTATION ON GITHUB.
%macro sys_pkill 2
	mov rax, 0x07 ; syscall sys_pkill
	mov rdi, %1   ; process_handle (size_t)
	mov rsi, %2   ; kill_code      (uint)
	syscall       ; the return value is stored in the rax register 
%endmacro
; [no documentation is available, behaviour unknown]

;SYS_EXIT
; Send signal to kernel to kill the user's process.
%macro sys_exit 1
	mov rax, 0x08 ; syscall sys_exit
	mov rdi, %1   ; exit_code (int)
	syscall
%endmacro
; [no documentation is available, behaviour unknown]

;SYS_UMEMCPY
; Places an adress in the user's pages.
%macro sys_umemcpy 1
	mov rax, 0x09 ; syscall sys_umemcpy
	mov rdi, %1   ; adr (long)
	syscall
%endmacro
; [no documentation is available, behaviour unknown]

; SYS_RALLOC
; allocates a region of memory for the user.
%macro sys_ralloc 2
	mov rax, 0x0A ; syscall sys_ralloc
	mov rdi, %1   ; adr (long)	
	mov rsi, %2   ; len (size_t)
	syscall 
%endmacro
; [no documentation is available, behaviour unknown]

;SYS_CREATE_EXEC_POLICY
%macro sys_create_exec_policy 3
	mov rax, 0x0B ; syscall sys_create_exec_policy
	mov rdi, %1   ; parent_policy (size_t)	
	mov rsi, %2   ; max_ram_bytes (size_t)	
	mov rdx, %3   ; rights 		  (uint64_t)
	syscall       ; the return value is stored in the rax register 
%endmacro
; On success returns handle for next execution policy, otherwise returns a negative error code.

;SYS_CREATE_PROCESS
; Creates a new process and child thread on the specified entry point.
%macro sys_create_process 4
	mov rax, 0x0C ; syscall sys_create_process
	mov rdi, %2   ; policy      (size_t)	
	mov rsi, %2   ; name        (c_str)	
	mov rdx, %3   ; entry_point (uint64_t)	
	mov r10, %4   ; stack_size  (size_t)
	syscall       ; the return value is stored in the rax register 
%endmacro	
; On success returns handle for new process otherwise returns a negative error code.

;SYS_CREATE_EMPTY_PROCESS
; Creates an ampty process.
%macro sys_create_empty_process 2
	mov rax, 0x0D ; syscall sys_create_empty_process
	mov rdi, %1   ; policy (size_t)	
	mov rsi, %2   ; name   (c_str)
	syscall		  ; the return value is stored in the rax register 
%endmacro
; NOT EMPLEMENTED DUE TO THE RETURN DESCRIPTION BEING DIFFICULT TO 
; UNDERSTAND, THIS SHOULD BE PATCHED IN THE FUTURE. IF NOT PATCHED YET
; CHECK THE DOCUMENTATION ON GITHUB.

;SYS_CREATE_THREAD
; creates a new thread associated to the parent process.
%macro sys_create_thread 3
	mov rax, 0x0E ; syscall sys_create_thread
	mov rdi, %1   ; parent_process_handle (size_t)	
	mov rsi, %2   ; entry_point           (uint64_t)
	mov rdx, %3   ;	stack_size            (size_t)
	syscall 	  ; the return value is stored in the rax register 
%endmacro
; NOT EMPLEMENTED DUE TO THE RETURN DESCRIPTION BEING DIFFICULT TO 
; UNDERSTAND, THIS SHOULD BE PATCHED IN THE FUTURE. IF NOT PATCHED YET
; CHECK THE DOCUMENTATION ON GITHUB.

;SYS_GETPID
; NOT IMPLEMENTED YET DO NOT USE, THIS SHOULD BE ADDED IN THE FUTURE IF NOT
; CHECK DOCUMENTATION ON GITHUB.
%macro sys_getpid 0
	mov rax, 0x0F ; syscall sys_getpid
	syscall       ; the return value is stored in the rax register 
%endmacro
;On success returns handle for new process otherwise returns a negative error code.

;SYS_GETID
; NOT IMPLEMENTED YET DO NOT USE, THIS SHOULD BE ADDED IN THE FUTURE IF NOT
; CHECK DOCUMENTATION ON GITHUB.
%macro sys_getid
	mov rax, 0x10 ; syscall sys_getpid
	syscall
%endmacro
;On success returns handle for the thread on the parent process otherwise returns a negative error code.

;SYS_BRK
; Allocates and deallocates pages of memory based on the the stack top current address, this syscall is identical to it's linux counterpart. 
%macro sys_brk 1
	mov rax, 0x11 ; syscall sys_brk
	mov rdi, %1   ; new_brk (uint64_t)
	syscall       ; the return value is stored in the rax register 
%endmacro
; On success returns new address for top of the stack otherwise returns a negative error code.

;SYS_WAITPID
; blocks a child process to the running process as well as all the thread that are attached to it.
%macro sys_waitpid 4
	mov rax, 0x12 ; syscall sys_waitpid
	mov rdi, %1   ; process_handle (size_t)	
	mov rsi, %2   ; options 	   (uint64_t)
	mov rdx, %3   ; wstatus 	   (int*)
	mov r10, %4   ;	posix_rusage   (rusage*)
	syscall	      ; the return value is stored in the rax register 
%endmacro
; On success returns the handle of the process when it terminates, otherwise returns a negative error code.

;SYS_WAITID
; block the execution of a sigle thread based on the options given
%macro sys_waitid 4
	mov rax, 0x13 ; syscall sys_waitid
	mov rdi, %1   ; thread_handle (size_t)
	mov rsi, %2   ;	options       (uint64_t)
	mov rdx, %3   ;	wstatus       (int*)
	mov r10, %4   ;	posix_rusage  (rusage*)
%endmacro
; On sucess returns handle to terminated thread otherwise returns a negative error code.

