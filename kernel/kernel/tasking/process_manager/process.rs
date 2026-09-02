//! Module de gestion de processus (kernel ou non).
//! Un processus stocke l'état de son execution global ainsi que ses données d'exécutions.

use alloc::collections::btree_set::BTreeSet;
use alloc::string::String;
use core::sync::atomic::{AtomicUsize, Ordering};
use super::super::thread_manager::thread::TId;
use crate::kernel::tasking::{TaskingError, TaskingResult};

/// Identifiant d'un processus.
/// Sert aux threads à se référer à leurs parents et au ProcessManager à se référer à ses
/// processus.
pub type PId = usize;

/// Renvoie l'identifiant du prochain processus à instancier.
static NEXT_PID: AtomicUsize = AtomicUsize::new(1usize);

/// Réprésente un processus de l'instance courante de l'os.
pub struct Process {
    pid: PId,
    name: String,
    kind: ProcessKind,
    state: ProcessState,
    address_space: AddressSpace,
    threads: BTreeSet<TId>,
}

impl Process {
    /// Instancie un nouveau processus kernel.
    ///
    /// # Argument
    /// * `name`: nom du nouveau processus kernel à instancier.
    pub fn new_kernel(name: impl Into<String>) -> Self {
        Self {
            pid: NEXT_PID.fetch_add(1usize, Ordering::Relaxed),
            name: name.into(),
            kind: ProcessKind::Kernel,
            state: ProcessState::Alive,
            address_space: AddressSpace::kernel(),
            threads: BTreeSet::new()
        }
    }

    /// Instancie un nouveau processus utilisateur.
    ///
    /// # Argument
    /// * `name`: nom du nouveau processus utilisateur à instancier.
    pub fn new_user(name: impl Into<String>) -> Self {
        Self {
            pid: NEXT_PID.fetch_add(1usize, Ordering::Relaxed),
            name: name.into(),
            kind: ProcessKind::User,
            state: ProcessState::Alive,
            address_space: AddressSpace::user(),
            threads: BTreeSet::new()
        }
    }

    /// Renvoie l'identifiant du processus courant.
    pub fn get_pid(&self) -> PId {
        self.pid
    }

    /// Associe un nouveau thread au processus courant.
    /// Ne fait rien si le processus courant est déjà associé au thread qu'on veut lui ajouter.
    pub fn add_thread(&mut self, tid: TId) -> TaskingResult<()> {
        if self.threads.contains(&tid) {
            return Err(TaskingError::AlreadyExists);
        }

        self.threads.insert(tid);
        Ok(())
    }

    /// Enlève un thread dans le processus courant.
    /// Renvoie une erreur si le thread à supprimer n'existe pas dans le processus.
    pub fn remove_thread(&mut self, tid: TId) -> TaskingResult<()> {
        if !self.threads.contains(&tid) {
            return Err(TaskingError::ThreadNotFound(tid));
        }

        self.threads.remove(&tid);
        Ok(())
    }

    /// Renvoie le type du processus courant.
    pub fn get_kind(&self) -> ProcessKind {
        self.kind
    }

    /// Renvoie l'état du processus courant.
    pub fn get_state(&self) -> ProcessState {
        self.state
    }

    /// Renvoie le nom du processus courant.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Renvoie la liste des threads associés au processus courant.
    pub fn get_threads(&self) -> &BTreeSet<TId> {
        &self.threads
    }
}

/// Représente le type de processus avec lequel on travaille.
/// Il peut être soit Kernel soit Utilisateur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessKind {
    Kernel,
    User
}

/// Représente l'état d'un processus, peut importe son type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Alive,
    Dead
}

/// Représente l'espace d'adressage courant d'un processus.
pub struct AddressSpace {
    is_user: bool
}

impl AddressSpace {
    /// Instancie un espace d'adressage réservé au kernel.
    pub fn kernel() -> Self {
        Self {
            is_user: false
        }
    }

    /// Instancie un espace d'adressage réservé à l'utilisateur.
    pub fn user() -> Self {
        Self {
            is_user: true
        }
    }

    /// Permet de savoir si l'espace d'adressage courant est réservé à l'utilisateur ou au kernel.
    pub fn is_user(&self) -> bool {
        self.is_user
    }
}
