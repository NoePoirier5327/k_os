//! Module d'abstraction du contexte d'exécution global du cpu.

pub trait CpuContext {
    /// Initialise le contexte d'exécution.
    fn init(&'static self);

    /// Met à jour la pile noyau courante.
    /// Utile pour les changements de threads.
    fn update_kernel_stack(&self, stack_top: u64);
}
