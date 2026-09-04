//! Module de gestion de processus (kernel ou non).
//! Un processus stocke l'état de son execution global ainsi que ses données d'exécutions.

use alloc::collections::btree_set::BTreeSet;
use alloc::string::String;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{PhysFrame, Size4KiB};
use core::sync::atomic::{AtomicUsize, Ordering};
use super::super::thread_manager::thread::TId;
use crate::kernel::Kernel;
use crate::tasker::{TaskerError, TaskerResult};
use crate::memory::user::new_user_pml4;

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
            address_space: AddressSpace::new(false),
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
            address_space: AddressSpace::new(true),
            threads: BTreeSet::new()
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

    /// Renvoie l'espace d'adressage du processus courant.
    pub fn get_address_space(&self) -> &AddressSpace {
        &self.address_space
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
    is_user: bool,
    pml4_frame: PhysFrame<Size4KiB>
}

impl AddressSpace {
    /// Instancie un nouvel espace d'adressage.
    ///
    /// # Arguments
    /// * `is_user`: détermine si l'espace d'adressage est réservé à l'utilisateur ou au kernel.
    pub fn new(is_user: bool) -> Self {
        let pml4_frame = match is_user {
            // Si c'est un processus utilisateur
            // on lui alloue une nouvelle pml4.
            true => new_user_pml4(),

            // Si c'est un processus kernel
            // on lui donne la pml4 kernel
            false => Kernel::on_instance().get_pml4_frame()
        };

        Self {
            is_user,
            pml4_frame
        }
    }

    /// Permet de savoir si l'espace d'adressage courant est réservé à l'utilisateur ou au kernel.
    pub fn is_user(&self) -> bool {
        self.is_user
    }

    /// Renvoie la frame physique pml4 de l'espace d'adressage courant.
    pub fn get_pml4_frame(&self) -> PhysFrame<Size4KiB> {
        self.pml4_frame
    }

    /// Echange la pml4 courante avec la pml4 de l'espace d'addressage courant.
    pub unsafe fn swap_pml4(&self) {
        let (current_frame, flags) = Cr3::read();

        if current_frame != self.pml4_frame {
            Cr3::write(self.pml4_frame, flags);
        }
    }
}
