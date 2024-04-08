use anyhow::ensure;
use essential_types::{
    intent::{Directive, Intent},
    slots::{state_len, Slots},
    Signed,
};
use utils::verify;

#[cfg(test)]
mod tests;

/// Maximum number of intents that of an intent set.
pub const MAX_INTENTS: usize = 99;
/// Maximum number of state read programs of an intent.
pub const MAX_STATE_READS: usize = 101;
/// Maximum size of state read programs of an intent.
pub const MAX_STATE_READ_SIZE: usize = 999;
/// Maximum number of constraint check programs of an intent.
pub const MAX_CONSTRAINTS: usize = 98;
/// Maximum size of constraint check programs of an intent.
pub const MAX_CONSTRAINT_SIZE: usize = 1001;
/// Maximum number of decision variables of the slots of an intent.
pub const MAX_DECISION_VARIABLES: u32 = 998;
/// Maximum number of state slots of an intent.
pub const MAX_NUM_STATE_SLOTS: usize = 1002;
/// Maximum length of state slots of an intent.
pub const MAX_STATE_LEN: u32 = 997;
/// Maximum size of directive of an intent.
pub const MAX_DIRECTIVE_SIZE: usize = 1003;

/// Trait for validating essential types.
/// Validation of these types is necessary to ensure invalid data does not reach the storage.
pub trait Validate {
    fn validate(&self) -> anyhow::Result<()>;
}

/// Validation for a signed set of intents.
/// Verifies the signature and then validates the intent set.
impl Validate for Signed<Vec<Intent>> {
    fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            verify(self.clone()),
            "Failed to verify intent set signature"
        );
        self.data.validate()
    }
}

/// Validation for a set of intents.
/// Checks the size of the set and then validates each intent.
impl Validate for Vec<Intent> {
    fn validate(&self) -> anyhow::Result<()> {
        ensure!(self.len() <= MAX_INTENTS, "Too many intents");
        for i in self {
            i.validate()?;
        }
        Ok(())
    }
}

/// Validation for a single intent.
/// Validates the slots, directive, state reads, and constraints.
impl Validate for Intent {
    fn validate(&self) -> anyhow::Result<()> {
        self.slots.validate()?;
        self.directive.validate()?;
        // TODO: impl Validate for Vec<StateReadBytecode> and Vec<ConstraintBytecode> are
        // not possible because of conflicting implementations of Vec<Vec<u8>>
        // consider changing them to 1-tuple structs
        ensure!(
            self.state_read.len() <= MAX_STATE_READS,
            "Too many state reads"
        );
        ensure!(
            self.state_read
                .iter()
                .all(|sr| sr.len() <= MAX_STATE_READ_SIZE),
            "State read too large"
        );
        ensure!(
            self.constraints.len() <= MAX_CONSTRAINTS,
            "Too many constraints"
        );
        ensure!(
            self.constraints
                .iter()
                .all(|c| c.len() <= MAX_CONSTRAINT_SIZE),
            "Constraint too large"
        );
        Ok(())
    }
}

/// Validaton for intent slots.
impl Validate for Slots {
    fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.decision_variables <= MAX_DECISION_VARIABLES,
            "Too many decision variables"
        );
        ensure!(
            self.state.len() <= MAX_NUM_STATE_SLOTS,
            "Too many state slots"
        );
        ensure!(
            state_len(&self.state).is_some(),
            "Invalid slots state length"
        );
        ensure!(
            state_len(&self.state).unwrap() <= MAX_STATE_LEN,
            "Slots state length too large"
        );
        Ok(())
    }
}

/// Validaton for intent directive.
impl Validate for Directive {
    fn validate(&self) -> anyhow::Result<()> {
        if let Directive::Maximize(program) | Directive::Minimize(program) = &self {
            ensure!(program.len() <= MAX_DIRECTIVE_SIZE, "Directive too large");
        }
        Ok(())
    }
}
