use crate::*;
pub use internal::*;
use std::time::Instant;

// Helps enforce the usage of `exit` by defining transitions outside of this module.
mod internal {
    pub use super::*;

    #[derive(Debug)]
    pub struct ReadyState {
        ready_count: u64,
        ready_start: Instant,
    }

    impl ReadyState {
        pub fn enter(inputs: ReadyStateInputs) -> Self {
            let ReadyStateInputs { ready_count } = inputs;

            Self {
                ready_count: ready_count + 1,
                ready_start: Instant::now(),
            }
        }

        pub fn exit(self) -> ReadyStateOutputs {
            let Self {
                ready_count,
                ready_start,
            } = self;

            tracing::info!("Spent {:?} in ready state.", ready_start.elapsed());

            ReadyStateOutputs { ready_count }
        }

        pub fn ready_count(&self) -> u64 {
            self.ready_count
        }
    }
}

#[derive(Debug)]
pub struct ReadyStateInputs {
    pub ready_count: u64,
}

#[derive(Debug)]
pub struct ReadyStateOutputs {
    pub ready_count: u64,
}

impl_state_transition_result! {
    pub enum ReadyStateTransitionResult {
        Stored(StoredState),
    }
}

impl ReadyState {
    pub fn store(self) -> ReadyStateTransitionResult {
        let ReadyStateOutputs { ready_count } = self.exit();

        ReadyStateTransitionResult::Stored(StoredState::enter(StoredStateInputs { ready_count }))
    }
}
