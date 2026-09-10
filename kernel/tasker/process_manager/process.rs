//! Module de gestion de processus (kernel ou non).
//! Un processus stocke l'état de son execution global ainsi que ses données d'exécutions.

use alloc::collections::btree_set::BTreeSet;
use alloc::string::String;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::arch::hal::memory::{Mapper, MapperTrait, new_user_mapper};
use super::super::thread_manager::thread::TId;
use crate::tasker::{TaskerError, TaskerResult};

/// Identifiant d'un processus.
/// Sert aux threads à se référer à leurs parents et au ProcessManager à se référer à ses
/// processus.
pub type PId = usize;

/// Renvoie l'identifiant du prochain processus à instancier.
static NEXT_PID: AtomicUsize = AtomicUsize::new(1usize);

/// Réprésente un processus de l'instance courante de l'os.
pub struct Process<'a> {
    pid: PId,
    name: String,
    kind: ProcessKind,
    state: ProcessState,
    threads: BTreeSet<TId>,
    user_mapper: Option<Mapper<'a>>
}

impl<'a> Process<'a> {
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
            threads: BTreeSet::new(),
            user_mapper: None,
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
            threads: BTreeSet::new(),
            user_mapper: Some(unsafe { new_user_mapper() })
        }
    }

    /// Renvoie l'identifiant du processus courant.
    pub fn get_pid(&self) -> PId {
        self.pid
    }

    /// Associe un nouveau thread au processus courant.
    /// Ne fait rien si le processus courant est déjà associé au thread qu'on veut lui ajouter.
    pub fn add_thread(&mut self, tid: TId) -> TaskerResult<()> {
        if self.threads.contains(&tid) {
            return Err(TaskerError::AlreadyExists);
        }

        self.threads.insert(tid);
        Ok(())
    }

    /// Enlève un thread dans le processus courant.
    /// Renvoie une erreur si le thread à supprimer n'existe pas dans le processus.
    pub fn remove_thread(&mut self, tid: TId) -> TaskerResult<()> {
        if !self.threads.contains(&tid) {
            return Err(TaskerError::ThreadNotFound(tid));
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

    /// Marque le processus courant comme mort.
    pub fn kill(&mut self) {
        self.state = ProcessState::Dead;
    }

    /// Renvoie une interface vers le mapper utilisateur courant si le processus est de type
    /// utilisateur.
    pub fn get_user_mapper(&mut self) -> TaskerResult<&mut dyn MapperTrait> {
        if let Some(mapper) = &mut self.user_mapper {
            return Ok(mapper)
        }

        Err(TaskerError::WrongProcessKind)
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
