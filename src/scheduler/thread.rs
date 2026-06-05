//! Implémentation d'un multitasking coopératif kernel level.
// TODO le rendre préemptif.
// TODO Implémenter le multithreading pour d'autres architextures.

use crate::println;
use super::SCHEDULER;
use alloc::boxed::Box;
use alloc::vec;
use core::arch::naked_asm;


pub type ThreadId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,      // Prêt à tourner, attend son tour dans le scheduler
    Busy,    // En cours d'exécution sur le CPU
    Blocked,    // En attente d'un événement (clavier, timer...)
    Dead,     // Terminé, attend que sa mémoire soit nettoyée
}

/// Type représentant un thread kernel zoned
#[repr(C)] // Force Rust à garder l'ordre des champs pour l'assembleur
pub struct Thread {
    // Métadonnées de gestion
    id : ThreadId,
    state : ThreadState,

    // Gestion de la pile (Stack)
    stack : Box<[u8]>,
    stack_pointer : u64,
    stack_base : u64,
    stack_size : usize,
}

impl Thread {
    /// Constructeur de thread mémoire.
    ///
    /// # Arguments
    /// * `id` : identifiant du thread à créer.
    /// * `entry_point_addr` : adresse de la fonction à associer au thread à créer.
    /// * `stack_size` : taille du stack à allouer à l'execution du processus du thread.
    pub fn new(id : ThreadId, entry_point_addr : usize, stack_size : usize) -> Self {
        // On créer une pile vide remplie de 0.
        let stack = vec![0u8; stack_size].into_boxed_slice();
        let stack_base = stack.as_ptr() as u64;
        let mut rsp = (stack_size as u64 + stack_base) & !0xF;

        unsafe {
            let mut stack_ptr = rsp as *mut u64;

            // On place la fonction de fin de thread.
            stack_ptr = stack_ptr.offset(-1);
            stack_ptr.write(thread_exit as *const () as u64);

            // On place l'adresse de retourne tout en haut de la pile.
            stack_ptr = stack_ptr.offset(-1);
            stack_ptr.write(entry_point_addr as u64);

            // On simule le stockage de registre processeur dans la pile
            // privée du tas.
            let num_registers = 6; 
            rsp = (stack_ptr as u64) - (num_registers * 8);
        }

        Self {
            id,
            state : ThreadState::Ready,
            stack,
            stack_pointer : rsp,
            stack_base,
            stack_size
        }
    }

    /// Accesseur du pointeur de pile du thread courant sous forme d'une référence
    /// mutable.
    pub fn get_stack_pointer_mut(&mut self) -> &mut u64 {
        &mut self.stack_pointer
    }

    /// Accesseur du pointeur de pile du thread courant.
    pub fn get_stack_pointer(&self) -> u64 {
        self.stack_pointer
    }

    /// Accesseur de l'état du thread courant.
    pub fn get_state(&self) -> ThreadState {
        self.state
    }

    /// Met l'état du thread courant à mort.
    fn kill(&mut self) {
        self.state = ThreadState::Dead;
    }

    /// Met l'état du thread courant à prêt.
    pub fn ready(&mut self) {
        self.state = ThreadState::Ready;
    }

    /// Met l'état du thread courant à occupé.
    pub fn busy(&mut self) {
        self.state = ThreadState::Busy;
    }
}

/// Fonction de fin de thread pour que ce dernier sache où finir sans créer
/// de triple fault.
///
/// # Panic
/// Cette fonction peut paniquer quand le thread auquel elle est attachée est
/// toujours vivant après que l'ordonnanceur l'ai forcé à être déalloué.
fn thread_exit() -> ! {
    println!("INFO : Current thread finished its job.");

    // On tue le thread appelant.
    let mut scheduler = SCHEDULER.lock();
    if let Some(current_thread_mutex) = scheduler.get_current_thread_mut() {
        current_thread_mutex.kill();
    }

    // Puis, on force le passage au thread suivant.
    scheduler.schedule();

    // Sécurité au cas ou il y a un problème.
    panic!("ERROR : From this point, I should be dead, something wrong might have happened.");
}

/// Intervertit le contexte d'execution de deux threads.
///
/// # Arguments
/// * `old_thread` : pointeur vers la pile du thread sortant.
/// * `new_thread` : pointeur vers la pile du thread prenant la main.
#[unsafe(naked)]
pub unsafe extern "C" fn swap_context(old_thread : *mut u64, new_thread : u64) {
    // Pour info, selon la convention standard de rust (system V AMD64 ABI),
    // les arguments sont stockés comme suit : 
    //      Argument 1 -> RDI
    //      Argument 2 -> RSI
    //      Argument 3 -> RDX
    //      Argument 4 -> RCX
    //      Argument 5 -> R8
    //      Argument 6 -> R9
    //  => L'utilisation de `extern "C"` force l'utilisation de cette convention.

    naked_asm!(
        // On pousse les registres d'executions du thread sortant dans sa pile.
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // On stock le pointeur de pile courant dans la pile du thread sortant.
        "mov [rdi], rsp",

        // On récupère le pointeur de pile du thread entrant.
        "mov rsp, rsi",

        // On récupère les registres d'executions du thread entrant.
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",

        "ret"
    )
}
