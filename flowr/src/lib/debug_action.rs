/// Represents the action the coordinator should take after a debugger interaction.
///
/// This replaces the previous `(display_next_output: bool, restart: bool)` tuple pattern,
/// making the three distinct states explicit and preventing accidental misuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugAction {
    /// Continue normal execution — no debugger intervention
    Continue,
    /// Step mode — display the next output before pausing again
    DisplayNextOutput,
    /// Reset execution from scratch (debugger requested a restart)
    Restart,
}

impl DebugAction {
    /// Returns `true` if the action requests a restart of flow execution
    pub(crate) fn should_restart(self) -> bool {
        self == Self::Restart
    }

    /// Returns `true` if the action requests displaying the next output (step mode)
    pub(crate) fn should_display(self) -> bool {
        self == Self::DisplayNextOutput
    }
}
