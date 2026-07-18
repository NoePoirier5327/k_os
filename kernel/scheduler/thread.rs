//! Implémentation d'un multitasking préemptif kernel level.
// TODO Implémenter le multithreading pour d'autres architextures.
// TODO Implémenter un temps d'execution max pour éviter les blocages (ex: boucles infinies).

//use crate::println;
use crate::interrupts::{PICS, InterruptIndex};
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
    /// * `entry_point_adr` : adresse de la fonction à associer au thread à créer.
    /// * `stack_size` : taille du stack à allouer à l'execution du processus du thread.
    pub fn new(id : ThreadId, entry_point_adr : usize, stack_size : usize) -> Self {
        // On créer une pile vide remplie de 0.
        let stack = vec![0u8; stack_size].into_boxed_slice();
        let stack_base = stack.as_ptr() as u64;
        let mut rsp = (stack_size as u64 + stack_base) & !0xF;

        unsafe {
            let mut stack_ptr = rsp as *mut u64;

            // On place la fonction de fin de thread en haut de la pile.
            stack_ptr = stack_ptr.offset(-1);
            stack_ptr.write(thread_exit as *const () as u64);

            // Puis la fonction que thread doit executer.
            stack_ptr = stack_ptr.offset(-1);
            stack_ptr.write(entry_point_adr as u64);

            // Enfin, le trampoline que le nouveau thread doit emprunter.
            stack_ptr = stack_ptr.offset(-1);
            let trampoline_adr = trampoline as *const () as usize;
            stack_ptr.write(trampoline_adr as u64);

            // On simule le stockage de registre processeur dans la pile
            // privée du tas.
            let num_registers = 15; 
            stack_ptr = stack_ptr.offset(-num_registers);
            rsp = stack_ptr as u64;
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
    //println!("INFO : Current thread finished its job.");

    {
        // On tue le thread appelant.
        let mut scheduler = SCHEDULER.lock();
        if let Some(current_thread_mutex) = scheduler.get_current_thread_mut() {
            current_thread_mutex.kill();
        }
    } // Le mutex est libéré ici

    // Puis, on force le passage au thread suivant.
    super::schedule();

    // Sécurité au cas ou il y a un problème.
    panic!("ERROR : From this point, I should be dead, something wrong might have happened.");
}

/// Fonction de trampoline permettant de réactiver les interruptions processeur et envoyer le signal
/// de fin d'interruption courante pour un thread qui vient juste d'être créé.
#[unsafe(naked)]
extern "C" fn trampoline() {
    naked_asm!(
        // On termine le traitement du tick courant.
        "call {notify_timer}",
        "sti",

        // On saute vers la fonction cible du thread
        "ret",
        notify_timer = sym notify_timer_handler
    )
}

/// Permet de signaler qu'on a fini de traiter le thread courant sans le faire à la main en
/// assembleur.
extern "C" fn notify_timer_handler() {
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.to_u8());
    }
}

/// Intervertit le contexte d'execution de deux threads.
///
/// # Arguments
/// * `old_thread` : pointeur vers la pile du thread sortant.
/// * `new_thread` : pointeur vers la pile du thread prenant la main.
///
/// # Safety
/// L'appelant doit s'assurer que les piles qu'il est train d'échanger ne sont pas
/// corrompus, auquel cas, il est responsable des fautes processeurs que cela entraînera.
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
        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rbp",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        
        "mov ecx, 0xC0000101", // IA32_GS_BASE MSR
        "rdmsr",
        "push rdx",
        "push rax",

        // On stock le pointeur de pile courant dans la pile du thread sortant.
        "mov [rdi], rsp",

        // On récupère le pointeur de pile du thread entrant.
        "mov rsp, rsi",

        "pop rax",
        "pop rdx",
        //"mov ecx, 0xC0000101",
        "wrmsr",

        // On récupère les registres d'executions du thread entrant.
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rbp",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",

        "ret"
    )
}
