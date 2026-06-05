//! Implémentation d'un multitasking coopératif kernel level.
// TODO le rendre préemptif.
// TODO Implémenter le multithreading pour d'autres architextures.

use crate::println;
use alloc::boxed::Box;
use alloc::vec;
use spin::mutex::Mutex;
use core::arch::naked_asm;


pub type ThreadId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,      // Prêt à tourner, attend son tour dans le scheduler
    Busy,    // En cours d'exécution sur le CPU
    Blocked,    // En attente d'un événement (clavier, timer...)
    Exited,     // Terminé, attend que sa mémoire soit nettoyée
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

    /// Accesseur du pointeur de pile du thread courant.
    pub fn get_stack_pointer(&self) -> u64 {
        self.stack_pointer
    }
}

/// Fonction de fin de thread pour que ce dernier sache où finir sans créer
/// de triple fault.
fn thread_exit() -> ! {
    println!("INFO : Current thread finished correctly.");
    loop {
        x86_64::instructions::hlt();
    }
}


/// Intervertit deux threads entre eux.
pub fn swap_threads(old_thread : &Mutex<Thread>, new_thread : &Mutex<Thread>) {
    let (old_stack_pointer, new_stack_pointer) = {
        // On mobilise la donnée des threads
        let mut old_guard = old_thread.lock();
        let mut new_guard = new_thread.lock();
       
        // On change les états d'éxecution
        old_guard.state = ThreadState::Ready;
        new_guard.state = ThreadState::Busy;
        
        // On récupère les pointeurs bruts sous forme d'adresses
        (&mut old_guard.stack_pointer as *mut u64, new_guard.stack_pointer)
    }; // La donnée des threads est libérée

    unsafe {
        swap_context(
            old_stack_pointer, 
            new_stack_pointer
        );
    }
}

/// Intervertit le contexte d'execution de deux threads.
///
/// # Arguments
/// * `old_thread` : pointeur vers la pile du thread sortant.
/// * `new_thread` : pointeur vers la pile du thread prenant la main.
#[unsafe(naked)]
unsafe extern "C" fn swap_context(old_thread : *mut u64, new_thread : u64) {
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
