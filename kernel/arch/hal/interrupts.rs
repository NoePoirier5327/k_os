//! Module contenant le code d'abstraction des interruptions processeur.

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

/// Interface générique de récuperation de contexte d'exécution à chaque interruption timer.
/// Utile pour le multiprocess.
pub trait InterruptionStackFrame {}
