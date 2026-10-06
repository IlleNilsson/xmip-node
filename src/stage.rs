//! The stages of the message path: receive, process, send.
//!
//! A stage is where a Message is on its path and what a scope names beneath
//! a node (`<node>/receive`); it is not what a node declares. A node
//! declares its roles, and a role says which stages it serves
//! ([`NodeRole::stages`](crate::NodeRole::stages); ADR-0056, amendment
//! 2026-10-01). The stage words are the scope segments, exact lowercase.
//!
//! No other language writes the words or the facts of a stage again: the
//! runtime's library forwards [`Stage::WORDS`], [`Stage::pausable`] and
//! [`Stage::location`] to the surfaces (`xmip_operate.h` section 7), and
//! `Xmip.Surface` calls those (ADR-0056, amendment 2026-09-24).

/// A stage of the message path, in path order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Stage {
    Receive,
    Process,
    Send,
}

impl Stage {
    /// Every stage, in message-path order.
    pub const ALL: [Stage; 3] = [Stage::Receive, Stage::Process, Stage::Send];

    /// The stage words, in message-path order: each stage's
    /// [`name`](Self::name), the segment a scope names it by.
    pub const WORDS: [&'static str; 3] = ["receive", "process", "send"];

    /// The stage's canonical word, as it appears in a scope, a flag and a
    /// published record.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Stage::Receive => "receive",
            Stage::Process => "process",
            Stage::Send => "send",
        }
    }

    /// The stage a word names, exactly and in lowercase, or `None` when no
    /// stage is called that.
    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|stage| stage.name() == word)
    }

    /// The stage a handoff goes on to: receive to process, process to send.
    /// Send closes the path and hands nothing on.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::Receive => Some(Self::Process),
            Self::Process => Some(Self::Send),
            Self::Send => None,
        }
    }

    /// Whether an operator may pause the stage: a Receive or a Send Location
    /// can be held; a Work Process runs off a subscription, and an operator
    /// pauses the Location that feeds it, not the Process itself.
    #[must_use]
    pub const fn pausable(self) -> bool {
        matches!(self, Self::Receive | Self::Send)
    }

    /// What a thing configured at the stage is called: a receive location,
    /// a Work Process, a send location (ADR-0027 clause 4).
    #[must_use]
    pub const fn location(self) -> &'static str {
        match self {
            Self::Receive => "receive location",
            Self::Process => "work process",
            Self::Send => "send location",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_are_the_stages_names_in_path_order() {
        assert_eq!(Stage::ALL.map(Stage::name), Stage::WORDS);
        assert_eq!(Stage::Receive.next(), Some(Stage::Process));
        assert_eq!(Stage::Send.next(), None);
    }

    #[test]
    fn a_location_can_be_paused_and_a_process_cannot() {
        assert_eq!(Stage::ALL.map(Stage::pausable), [true, false, true]);
        assert_eq!(
            Stage::ALL.map(Stage::location),
            ["receive location", "work process", "send location"]
        );
    }
}
