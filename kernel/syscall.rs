// Codes de retour d'un syscall calqués sur les conventions linux.
/// Syscall demandé non implémenté.
const ENOSYS : i64 = 38;

/// Retour correct de syscall.
const ESUCCESS : i64 = 0;

/// Erreur d'execution
const EFAILED : i64 = -1;

/// Argument non conforme
const ARGERROR : i64 = -2;

/// Taille max des chaînes de caractères affichable.
const MAX_SYS_DISP_STR_SIZE: usize = 2000_usize;

/// Représente la fonction de gestion d'un appel système.
type SyscallFn = fn(u64,u64,u64) -> i64;

const SYSCALL_TABLE : [Option<SyscallFn>; 2] = [
    Some(sys_disp),
    Some(sys_dispcolor)
];

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
///
/// # Safety
#[no_mangle]
pub unsafe extern "sysv64" fn generic_syscall_dispatcher(
    id : u64,
    arg1 : u64,
    arg2 : u64,
    arg3 : u64
) -> i64 {
    match SYSCALL_TABLE.get(id as usize) {
        Some(Some(handler)) => handler(arg1, arg2, arg3),

        _ => {
            crate::disp_warning!("Syscall `0x{:x}` doesn't exist.", id);
            ENOSYS
        }
    }
}


/// Implémentation de l'appel système d'affichage sur le buffer vga.
///
/// # Arguments
/// - `msg` : pointeur vers le premier caractère du message à afficher.
/// - `msg_len` : taille du message à afficher.
/// - `dummy` : argument inutile.
fn sys_disp(msg: u64, msg_len: u64, _dummy: u64) -> i64 {
    use crate::memory::string::string_extraction::{extract_str_with_len, StringExtractionError};
    let to_disp = match unsafe {
        extract_str_with_len(msg, msg_len as usize, MAX_SYS_DISP_STR_SIZE)
    } {
        Ok(msg) => msg,
        Err(e) => {
            match e {
                StringExtractionError::InvalidStringFormat => {
                    crate::disp_warning!("Invalid string format detected.");
                }

                StringExtractionError::ServiceDenial => {
                    crate::disp_error!("Service denial tentative detected.");
                }
            }

            return ARGERROR
        }
    };
        

    crate::print!("{}", to_disp);
    ESUCCESS
}

/// Implémentation de l'appel système de changement de couleur sur le buffer vga.
///
/// #Arguments
/// - `ft_color` : couleur du texte.
/// - `bg_color` : couleur de fond du texte.
/// - `dummy`    : argument inutile.
fn sys_dispcolor(ft_color: u64, bg_color: u64, _dummy: u64) -> i64 {
    if ft_color > 15 || bg_color > 15 {
        return ARGERROR;
    }

    crate::vga_buffer::set_writer_color(
        crate::vga_buffer::Color::from_code_to_color(ft_color as u8),
        crate::vga_buffer::Color::from_code_to_color(bg_color as u8)
    );

    ESUCCESS
}
