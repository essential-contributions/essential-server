use anyhow::ensure;
use essential_types::{
    intent::{Directive, Intent},
    slots::state_len,
    Signed,
};
use utils::verify;

#[cfg(test)]
mod tests;

/// Maximum number of intents that of an intent set.
pub const MAX_INTENTS: usize = 100;
/// Maximum number of state read programs of an intent.
pub const MAX_STATE_READS: usize = 100;
/// Maximum size of state read programs of an intent in bytes.
pub const MAX_STATE_READ_SIZE_IN_BYTES: usize = 10_000;
/// Maximum number of constraint check programs of an intent.
pub const MAX_CONSTRAINTS: usize = 100;
/// Maximum size of constraint check programs of an intent in bytes.
pub const MAX_CONSTRAINT_SIZE_IN_BYTES: usize = 10_000;
/// Maximum number of decision variables of the slots of an intent.
pub const MAX_DECISION_VARIABLES: u32 = 100;
/// Maximum number of state slots of an intent.
pub const MAX_NUM_STATE_SLOTS: usize = 1000;
/// Maximum length of state slots of an intent.
pub const MAX_STATE_LEN: u32 = 1000;
/// Maximum size of directive of an intent.
pub const MAX_DIRECTIVE_SIZE: usize = 1000;

/// Validation for a signed set of intents.
/// Verifies the signature and then validates the intent set.
/// Checks the size of the set and then validates each intent.
pub fn validate_intents(intents: &Signed<Vec<Intent>>) -> anyhow::Result<()> {
    ensure!(verify(intents), "Failed to verify intent set signature");
    ensure!(intents.data.len() <= MAX_INTENTS, "Too many intents");
    for i in &intents.data {
        validate_intent(i)?;
    }
    Ok(())
}

/// Validation for a single intent.
/// Validates the slots, directive, state reads, and constraints.
pub fn validate_intent(intent: &Intent) -> anyhow::Result<()> {
    // Validate slots
    ensure!(
        intent.slots.decision_variables <= MAX_DECISION_VARIABLES,
        "Too many decision variables"
    );
    ensure!(
        intent.slots.state.len() <= MAX_NUM_STATE_SLOTS,
        "Too many state slots"
    );
    let len = state_len(&intent.slots.state);
    ensure!(len.is_some(), "Invalid slots state length");
    ensure!(
        len.unwrap() <= MAX_STATE_LEN,
        "Slots state length too large"
    );

    // Validate directive
    if let Directive::Maximize(program) | Directive::Minimize(program) = &intent.directive {
        ensure!(program.len() <= MAX_DIRECTIVE_SIZE, "Directive too large");
    }

    // Validate state reads
    ensure!(
        intent.state_read.len() <= MAX_STATE_READS,
        "Too many state reads"
    );
    ensure!(
        intent
            .state_read
            .iter()
            .all(|sr| sr.len() <= MAX_STATE_READ_SIZE_IN_BYTES),
        "State read too large"
    );

    // Validate constraints
    ensure!(
        intent.constraints.len() <= MAX_CONSTRAINTS,
        "Too many constraints"
    );
    ensure!(
        intent
            .constraints
            .iter()
            .all(|c| c.len() <= MAX_CONSTRAINT_SIZE_IN_BYTES),
        "Constraint too large"
    );
    Ok(())
}
