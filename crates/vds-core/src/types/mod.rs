//! The artefact types. VDS holds exactly eight artefact kinds (VDS S-4(1)).
//!
//! Every closed set in the specification is an `enum` here rather than a
//! validated string. That is the strongest available form of the rule: VDS S-5(3)
//! says a record "may not invent a tenth" state, and with [`State`] as an enum a
//! tenth state is not a validation failure but an unrepresentable value. The same
//! goes for the seven lifecycle statuses (VDS S-5(4)) and the closed registry of
//! ten proof kinds (VDS S-7(5)).
//!
//! None of these types can hold a design REALISATION. There is no colour field,
//! no length field, no font field, no duration field and no easing field anywhere
//! in this module, and `no_stored_values` re-checks that claim against the bytes
//! on disk rather than trusting it (VDS S-2(8)).

mod component;
mod lock;
mod pin;
mod proof;
mod submission;
mod warrant;

pub use component::*;
pub use lock::*;
pub use pin::*;
pub use proof::*;
pub use submission::*;
pub use warrant::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The nine states, fixed by VDS S-5(3). A record may require a subset. It may
/// not invent a tenth, and here it cannot.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Default,
    Hover,
    Focus,
    Active,
    Selected,
    Disabled,
    Loading,
    Error,
    Success,
}

impl State {
    /// The nine, in the specification's order. Every report that lists states
    /// uses this order, so two runs never disagree about presentation.
    pub const ALL: [State; 9] = [
        State::Default,
        State::Hover,
        State::Focus,
        State::Active,
        State::Selected,
        State::Disabled,
        State::Loading,
        State::Error,
        State::Success,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            State::Default => "default",
            State::Hover => "hover",
            State::Focus => "focus",
            State::Active => "active",
            State::Selected => "selected",
            State::Disabled => "disabled",
            State::Loading => "loading",
            State::Error => "error",
            State::Success => "success",
        }
    }

    pub fn parse(raw: &str) -> Option<State> {
        State::ALL.into_iter().find(|s| s.as_str() == raw)
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The lifecycle at VDS S-5(4). A directed path where skipping is forbidden.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Proposed,
    Designed,
    Registered,
    Built,
    Verified,
    Deprecated,
    Retired,
}

impl Status {
    pub const PATH: [Status; 7] = [
        Status::Proposed,
        Status::Designed,
        Status::Registered,
        Status::Built,
        Status::Verified,
        Status::Deprecated,
        Status::Retired,
    ];

    /// A component in one of these states is registered for composition
    /// purposes. `proposed` and `designed` are NOT: a design that is merely
    /// drawn is exactly what the anti-drift proof exists to catch being used.
    pub const ENFORCEABLE: [Status; 3] = [Status::Registered, Status::Built, Status::Verified];

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Proposed => "proposed",
            Status::Designed => "designed",
            Status::Registered => "registered",
            Status::Built => "built",
            Status::Verified => "verified",
            Status::Deprecated => "deprecated",
            Status::Retired => "retired",
        }
    }

    pub fn parse(raw: &str) -> Option<Status> {
        Status::PATH.into_iter().find(|s| s.as_str() == raw)
    }

    pub fn index(self) -> usize {
        Status::PATH
            .iter()
            .position(|s| *s == self)
            .expect("every Status is on the path")
    }

    pub fn is_enforceable(self) -> bool {
        Status::ENFORCEABLE.contains(&self)
    }

    /// The one status this status may advance to by an ordinary `set-status`.
    ///
    /// Returns `None` at `verified`, because the next two transitions are
    /// deprecation and retirement, and VDS S-9(6) makes those three phases that
    /// cannot be compressed into a status assignment.
    pub fn ordinary_successor(self) -> Option<Status> {
        match self {
            Status::Verified | Status::Deprecated | Status::Retired => None,
            other => Some(Status::PATH[other.index() + 1]),
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nine_states_round_trip_through_their_wire_form() {
        for state in State::ALL {
            let text = serde_json::to_string(&state).unwrap();
            assert_eq!(text, format!("\"{}\"", state.as_str()));
            assert_eq!(serde_json::from_str::<State>(&text).unwrap(), state);
        }
    }

    #[test]
    fn a_tenth_state_is_unrepresentable() {
        assert!(serde_json::from_str::<State>("\"sparkling\"").is_err());
        assert!(State::parse("sparkling").is_none());
    }

    #[test]
    fn the_lifecycle_successor_stops_before_deprecation() {
        assert_eq!(Status::Proposed.ordinary_successor(), Some(Status::Designed));
        assert_eq!(Status::Built.ordinary_successor(), Some(Status::Verified));
        assert_eq!(
            Status::Verified.ordinary_successor(),
            None,
            "VDS S-9(6): retirement is three phases and is not a status assignment"
        );
        assert_eq!(Status::Retired.ordinary_successor(), None);
    }

    #[test]
    fn only_registered_built_and_verified_are_enforceable() {
        for status in Status::PATH {
            assert_eq!(
                status.is_enforceable(),
                matches!(status, Status::Registered | Status::Built | Status::Verified),
                "{status} classified wrongly"
            );
        }
    }
}
