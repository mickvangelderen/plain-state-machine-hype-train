So every program is a state machine. Why should you want to represent state machines more explicitly? This is the topic that this document explores. 

## Starting out

Consider skimming through [this stanford page](https://cs.stanford.edu/people/eroberts/courses/soco/projects/2004-05/automata-theory/basics.html#:~:text=An%20automaton%20in%20which%20the,and%20a%20state%20transition%20function.) if you are not quite sure what a deterministic finite automaton is.
It is not necessary to have all theory understood and top of mind to follow this article, but it is good to know that there is a theoretical basis to build upon.

## Basic implementation

```rust
enum State {
    A,
    B,
}

enum Symbol {
    Stay,
    Switch,
}

impl State {
    fn transition(self, symbol: Symbol) -> Self {
        match (self, symbol) {
            (Self::A, Symbol::Stay) => Self::A,
            (Self::A, Symbol::Switch) => Self::B,
            (Self::B, Symbol::Stay) => Self::B,
            (Self::B, Symbol::Switch) => Self::A,
        }
    }
}
```

Awesome, we're done. We implemented a finite state machine that stays in the same state when we transition with  `Symbol::Stay` and switches to the other state when we transition with `Symbol::Switch`.
Now what?

## Desired guarantees and features

When state machines grow, we'd like to manage the complexity and limit the possibility of making mistakes in the implementation.
In particular, some of the things that we might want to have are:

- ensuring a piece of code always runs upon entering or leaving a state,
- limiting the possible states returned by a transition implementation,
- being able to generate a diagram of the states and allowed transitions from the source code,
- being able to pass around some context that all states can access, and
- allow transitions to be asynchronous.

> TODO(mickvangelderen): Revisit.

Given that we are doing engineering, there is more than one solution to these features, including simply not solving or supporting them.
Each solution will have different advantages and disadvantages.
From here on out, some parts of the text will be subjective.
The opinions are based on many years of software engineering experience, but they are still opinions and therefore possibly wrong and definitely subject to change.

## Running example

The state machine that is used in this repository has two states: `Stored` and `Ready`.
The state machine does two things:

1. keep track and log how much time we have spent in a state.
2. count how often we have entered the `Ready` state.

Here are the states and their fields:

```rust
use std::time::Instant;

pub struct StoredState {
    ready_count: u64,
    stored_start: Instant
}

pub struct ReadyState {
    ready_count: u64,
    ready_start: Instant,
}
```

We will need an `enum` to allow storing different states in a single value:

```rust
pub enum State {
    Stored(StoredState),
    Ready(ReadyState),
}
```

We can define the symbols and the transition function:

```rust
pub enum Symbol {
    Store,
    Ready,
}

impl State {
    pub fn transition(self, symbol: Symbol) -> Self {
        match (self, symbol) {
            (Self::Stored(StoredState { ready_count, stored_start }), Symbol::Ready) => {
                info!("Spent {:?} in stored state.", stored_start.elapsed());
                Self::Ready(ReadyState { ready_count: ready_count + 1, ready_start: Instant::now() })
            },
            // ...
        }
    }
}
```

This implementation is fine for this small state machine. 
When the state machine and the complexity of the transitions increases, this will become messy.

## Transitions as methods

We can pull the each transition implementation into a method defined on the state itself to keep the transition implementation simple.

```rust
impl StoredState {
    pub fn ready(self) -> State {
        let StoredState { ready_count, stored_start } = self;
        info!("Spent {:?} in stored state.", stored_start.elapsed());
        State::Ready(ReadyState { ready_count: ready_count + 1, ready_start: Instant::now() })
    }
}

impl State {
    pub fn transition(self, symbol: Symbol) -> Self {
        match (self, symbol) {
            (Self::Stored(state), Symbol::Ready) => state.ready(),
            // ...
        }
    }
}
```

In fact, we can even define all `Symbol::Ready` transitions as a method on `State`:

```rust
impl State {
    pub fn transition(self, symbol: Symbol) -> Self {
        match symbol {
            Symbol::Ready => self.ready(),
            // ...
        }
    }

    pub fn ready(self) -> Self {
        match self {
            Self::Stored(state) => state.ready(),
            other => other, // The other states (Ready) do not support this symbol and so we just stay in the Ready state.
        }
    }
}
```

Now it turns out that we don't actually need the `transition` function and `Symbol` type to implement a state machine.
They are still useful if we need to represent operations performed on the state machine where we do not know at compile time what those operations are.
This can happen if we read data from disk or if we need to communicate between threads or tasks.

## Limiting valid transitions at compile-time

The only valid transition of the `Stored` state is to the `Ready` state.
We can represent this declaration of the `StoredState::ready` method.

```rust
impl StoredState {
    pub fn ready(self) -> ReadyState { // <- was `State`
        // ...
    }
}

impl State {
    pub fn ready(self) -> Self {
        match self {
            Self::Stored(state) => State::Ready(state.ready()), // <- was `state.ready()`
            // ...
        }
    }
}
```

> TODO(mickvangelderen): Not entirely sure that you would design a state machine this way? Maybe you should try to capture all of this in the symbols. Maybe that explodes the number of states you need (NFA -> DFA).

In a real world application it may make sense to design a state machine where for one of the operations or symbols, the next state is non-deterministic.
If the transition relies on a fallible operation or random chance, the next state may be a subset of all possible states.
Instead of immedietly reverting to use `State`, we can define a new type `StateSubset` which only contains the subset of valid states coming out of that transition.

```rust
enum StoredStateReadyTransitionResult {
    Ready,
    // You would add the other possible states here.
}

impl From<StoredStateReadyTransitionResult> for State {
    // Convert from subset to superset.
}
```

What does this give us besides more code?
With this implementation no one can change the possible transitions in the state machine without changing both 1) the transition implementation and the 2) the transition result type.
The additional friction should help anyone attempting to make this change think hard about whether they should be doing this.

## Communicating back transition rejections

In the previous section we had to handle a transition that we consider to be "invalid".
In some cases it can be nice to allow the initiator of the transition know that they tried something strange.
One way of doing so is through the `Result<T, E>` type.

```rust
impl State {
    pub fn ready(self) -> Result<Self, Self> { // <- was `Self`
        match self {
            Self::Stored(state) => Ok(State::Ready(state.ready())), // <- wrap in Ok(...)
            other => Err(other), // <- wrap in Err(...)
        }
    }
}
```

This small change allows us to communicate back to the caller that they attempted to make a transition that we consider invalid.

## State entry and exit hooks

In our example we increment the `ready_count` value inside of the `StoredState::ready` transition implementation.
This means that if we add a third state that can also transition to `Ready`, we will need to duplicate the code that increments `ready_count`.
Being able to run some code upon entering or leaving a state is very useful if it needs to happen regardless of the state you are coming from and of the symbol or operation that caused it.

Since our state machine implementation consumes states by value and produce a new state when we transition, we can move the code that needs to happen when entering a state into a constructor.
If the only way to construct that state is through a single constructor, we guarantee that this code will be run.

```rust
impl ReadyState {
    pub fn enter(ready_count: u64) -> Self {
        Self {
            ready_count: ready_count + 1,
            ready_start: Instant::now(),
        }
    }
}
```

To make it impossible for transition implementations to create a `ReadyState` through direct instantiation, we can move the transition implementations to another module that does not have access to at least one private field in the state.

Similarly, to always log how long we spent in the `Ready` state, we would like to run some code regardless of how we exit the `Ready` state.
This can be realised by allowing only a single method to move values out of `ReadyState`.

```rust
impl ReadyState {
    pub fn exit(self) -> u64 {
        Self {
            ready_count,
            ready_start,
        } = self;

        info!("Spent {:?} inside ready state", ready_start.elapsed());

        ready_count
    }
}
```

Of course, this is more effective when the fields inside of the state are not `Copy`.
You could even imagine passing a zero-sized token around all the states and only allowing the initial state of the state machine to create this token.

Most of the time your states will have more than a few parameters.
The enter and exit values can be represented by some additional structs:

```rust
struct StoredStateInputs {
    ready_count: u64,
}

struct StoredStateOutputs {
    ready_count: u64,
}
```

You would use use these definitions as the argument and return type of the `enter` and `exit` methods respectively.

## Passing around context

Imagine that every state might need to access a database.
In this case the database connection type can simply be added to every state definition and passed around.
If there are multiple values that need to be passed around, group them in a `struct Context { ... }` or some other way.
I would recommend trying hard to keep the nesting of objects to a minimum.
The more grouping your introduce, the less flexible the software becomes.

## Generating a state machine diagram from the source code

If you are to maintain a piece of software that contains a large state machine it is nice to be able to visualize it.
Keeping documentation up-to-date manually is quite hard and it is easy to make mistakes.

While I haven't figured out how to do this, I think we can get pretty far by limiting the possible state variants returned by transition implementations.
Perhaps we can just interpret the source code or maybe use prodedural macros.
Sorry for not being able to demo this yet.

## Asynchronous transitions

In some cases it may be useful to support asynchronous transitions. 
However, personally I am a bit wary of this concept.
The reason for this is that futures are state machines of themselves.
This means that you are mixing an explicit state machine with implicitly defined state machines.
By keeping the state machine synchronous, you can always respond to queries that want to try and do something with the state machine.
In case of async transitions, you will move the state into the async future and keep it there until it resolves, making it impossible to do anything with the state in the mean time.

## Implementation

The complete implementation can be found in the package defined in this repository.
It may see some refinement after the initial writing (which I am doing right now).
The [`lib.rs`](src/lib.rs) and [`stored.rs`](src/stored.rs) files are documented most thoroughly with some motivations for the implementation.

## Challenges

1. Change the state machine to prevent transitioning to the ready state more than 3 times.
2. Add a `Token` type that can only be constructed by the initial state `Stored` and prevent instantiating `ReadyState` without a token.
3. Modify the implementation to show the name of the current state on the command line before asking what operation to perform.
