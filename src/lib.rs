mod ready;
mod stored;

pub use ready::*;
pub use stored::*;

#[derive(Debug)]
pub enum State {
    Stored(StoredState),
    Ready(ReadyState),
}

// Consider definig a constructor (with or without arguments, whatever you need) only for the
// initial state.
impl Default for State {
    fn default() -> Self {
        Self::Stored(StoredState::enter(StoredStateInputs { ready_count: 0 }))
    }
}

impl State {
    // You could create some representation of the state here which you can share or display.
    pub fn name(&self) -> &'static str {
        match self {
            State::Stored(_) => "stored",
            State::Ready(_) => "ready",
        }
    }

    pub fn ready(self) -> Result<Self, Self> {
        match self {
            State::Stored(state) => Ok(state.ready().into()),
            _ => Err(self),
        }
    }

    pub fn store(self) -> Result<Self, Self> {
        match self {
            State::Ready(state) => Ok(state.store().into()),
            _ => Err(self),
        }
    }
}

/// Provides From<$TransitionResult> for State.
#[macro_export]
macro_rules! impl_state_transition_result {
    (pub enum $TransitionResult: ident { $($Variant: ident ($State: ty)),* $(,)? }) => {
        #[derive(Debug)]
        pub enum $TransitionResult {
            $($Variant($State)),*
        }

        impl From<$TransitionResult> for State {
            fn from(value: $TransitionResult) -> Self {
                match value {
                    $($TransitionResult::$Variant(state) => Self::$Variant(state)),*
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let state = State::Stored(StoredState::enter(StoredStateInputs { ready_count: 0 }));
        let state = state
            .store()
            .expect_err("can not transition from stored to stored");
        let state = state
            .ready()
            .expect("should be able to transition from stored to ready");
        let ready_state = match state {
            State::Ready(ref state) => state,
            _ => panic!("state should be ready"),
        };
        assert_eq!(1, ready_state.ready_count());
        let state = state
            .store()
            .expect("should be able to transition from ready to stored");
        _ = state;
    }
}
