//! Module contenant le code d'abstraction des interruptions processeur.

#[cfg(target_arch = "x86_64")]
pub use crate::arch::x86_64::interrupts::new_kernel_interruption_stack_frame;

#[cfg(target_arch = "x86_64")]
pub use crate::arch::x86_64::interrupts::new_user_interruption_stack_frame;

/// Type de l'interruption que l'on souhaite cibler.
pub enum InterruptionType {
    Timer,
    Keyboard
}

/// Interface de contrôle d'interruptions.
pub trait InterruptionController {
    /// Initialise le contrôleur sous-jacent.
    fn init(&mut self);

    /// Active l'interruption en paramètre.
    fn enable(&mut self, i_type: InterruptionType);

    /// Désactive l'interruption en paramètre.
    fn disable(&mut self, i_type: InterruptionType);

    /// Marque la fin de l'interruption en paramètre.
    fn end_of_interrupt(&mut self, i_type: InterruptionType);
}
