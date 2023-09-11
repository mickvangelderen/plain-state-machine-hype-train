use crate::*;
pub use internal::*;
use std::time::Instant;

mod internal {
    use super::*;

    #[derive(Debug)]
    pub struct StoredState {
        ready_count: u64,
        stored_start: Instant,
    }

    // This implementation block can access private fields in StoredState. Only add methods here
    // that need this level of access.
    impl StoredState {
        /// This method needs to be called to construct an instance of the state. This means it will
        /// always be called when entering this state.
        pub fn enter(inputs: StoredStateInputs) -> Self {
            let StoredStateInputs { ready_count } = inputs;

            Self {
                ready_count,
                stored_start: Instant::now(),
            }
        }

        /// This method needs to be called when transitioning away from the stored state because it
        /// is the only way to move out of the private fields. This guarantees that this code will
        /// always be called when transitioning away from this state.
        pub fn exit(self) -> StoredStateOutputs {
            let Self {
                ready_count,
                stored_start,
            } = self;

            tracing::info!("Spent {:?} in stored state.", stored_start.elapsed());

            StoredStateOutputs { ready_count }
        }

        pub fn ready_count(&self) -> u64 {
            self.ready_count
        }
    }
}

/// This defines the inputs required to enter the StoredState.
#[derive(Debug)]
pub struct StoredStateInputs {
    pub ready_count: u64,
}

/// This defines the outputs provided when leaving the StoredState.
#[derive(Debug)]
pub struct StoredStateOutputs {
    pub ready_count: u64,
}

// Instead of returning any `State`, we define this type which contains only a subset of `State`
// variants and use it in our transition method definitions to statically guarantee we will only
// return any of these states.
//
// In this example we can only transition from `Stored` to `Ready` so we
// might as well use the `ReadyState` as a return parameter. If there are multiple different
// operations that can return different subsets of State variants, you can declare multiple
// transition result types.
impl_state_transition_result! {
    pub enum StoredStateTransitionResult {
        Ready(ReadyState),
    }
}

// This separate implementation block for the StoredState is placed outside of the module so we
// guarantee that we can not access the private fields. This is necessary to enforce calling the
// `exit` method.
impl StoredState {
    pub fn ready(self) -> StoredStateTransitionResult {
        // This will not compile, which is the intention, because the fields are inaccessible here.
        // let Self {
        //     ready_count,
        //     stored_start,
        // } = self;

        let StoredStateOutputs { ready_count } = self.exit();

        // The associated function ReadyState::enter takes care of incrementing the ready count so
        // that it always happens, regardless of which state we are coming from.
        StoredStateTransitionResult::Ready(ReadyState::enter(ReadyStateInputs { ready_count }))
    }
}

// We can easily write tests for a single state without worrying too much about other states. As
// long as we can construct the state inputs, we're good.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let state = StoredState::enter(StoredStateInputs { ready_count: 0 });
        assert_eq!(
            0,
            state.ready_count(),
            "entering the stored state should not modify the ready count"
        );
        let state = state.ready();
        assert!(
            matches!(state, StoredStateTransitionResult::Ready(_)),
            "should be able to transition to the ready state"
        );
    }
}
