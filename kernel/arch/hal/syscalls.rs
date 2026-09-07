//! Module d'abstraction des appels systèmes du kernel.

pub trait SyscallInterface {
    /// Configure l'intercepteur d'appels.
    fn init(&self);

    /// Met à jour la pile noyau utilisée lors de la transition du mode utilisateur vers le mode
    /// noyau.
    fn update_kernel_stack(&self, stack_top: u64);
}
