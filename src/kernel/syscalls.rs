use core::arch::asm;


// Codes de gestion des syscalls.
/// Syscall d'écriture dans la console.
const SYS_DISP : u64 = 0;

// Codes de retour d'un syscall calqués sur les conventions linux.
/// Syscall demandé non implémenté.
const ENOSYS : u64 = 38;

/// Retour correct de syscall.
const ESUCCESS : u64 = 0;


/// Dispatcher d'appels système, appel les fonctions kernels correspondantes au syscall courant.
///
/// # Arguments
/// * `id` : identifiant de l'appel système demandé.
/// * `arg1` : 1er argument du syscall.
/// * `arg2` : 2nd argument du syscall.
/// * `arg3` : 3ème argument du syscall.
///
/// # Return
/// Renvoie un code permettant de connaître le resultat de l'appel.
pub unsafe extern "sysv64" fn syscall_dispatcher(
    id : u64,
    arg1 : u64,
    arg2 : u64,
    arg3 : u64
) -> u64 {
    match id {
        SYS_DISP => {
            let to_disp = unsafe {
                super::vga_buffer::extract_str_from_adr(arg1, arg2)
                    .expect("Failed to extract the desired string from the ram")
            };

            super::vga_buffer::_print(format_args!("{}", to_disp))
        }

        _ => {
            crate::disp_warning!("Le syscall {} n'existe pas.", id);
            ENOSYS
        }
    }
}
