//! Self-contained UML sequence-diagram model.
//!
//! A sequence diagram is a fundamentally different shape from a flowchart:
//! **participants** own vertical **lifelines**, and time runs top-to-bottom as
//! an ordered list of **messages** between lifelines. Activation bars are
//! inferred from sync-call / return pairing. This module is independent of the
//! flowchart `engine` and is exported via its own tools.

pub mod export;

use serde::{Deserialize, Serialize};

use crate::error::FlowError;

/// A participant (object/actor) owning a lifeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub label: String,
    /// Render as a UML actor (stick figure) rather than a box.
    #[serde(default)]
    pub actor: bool,
}

/// Message arrow style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKind {
    /// Synchronous call (solid line, filled arrowhead).
    Sync,
    /// Asynchronous message (solid line, open arrowhead).
    Async,
    /// Return / reply (dashed line, open arrowhead).
    Return,
    /// Creation message (dashed line to a created participant).
    Create,
    /// Destruction message (ends a lifeline with an X).
    Destroy,
}

impl MessageKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "sync" | "call" | "->>" => Some(Self::Sync),
            "async" | "signal" | "->" => Some(Self::Async),
            "return" | "reply" | "-->>" | "-->" => Some(Self::Return),
            "create" | "new" => Some(Self::Create),
            "destroy" | "delete" => Some(Self::Destroy),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Sync => "sync",
            Self::Async => "async",
            Self::Return => "return",
            Self::Create => "create",
            Self::Destroy => "destroy",
        }
    }
    /// Mermaid arrow token between participants.
    pub fn mermaid(self) -> &'static str {
        match self {
            Self::Sync => "->>",
            Self::Async => "-)",
            Self::Return => "-->>",
            Self::Create => "->>",
            Self::Destroy => "-x",
        }
    }
    pub fn dashed(self) -> bool {
        matches!(self, Self::Return | Self::Create)
    }
}

/// A single message from one lifeline to another (or a self-message).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: String,
    #[serde(default = "default_sync")]
    pub kind: MessageKind,
}

fn default_sync() -> MessageKind {
    MessageKind::Sync
}

/// A full sequence diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sequence {
    pub title: Option<String>,
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
}

impl Sequence {
    pub fn new() -> Self {
        Self {
            title: None,
            participants: Vec::new(),
            messages: Vec::new(),
        }
    }

    fn has_participant(&self, id: &str) -> bool {
        self.participants.iter().any(|p| p.id == id)
    }

    /// Add a participant. Errors on empty or duplicate id.
    pub fn add_participant(&mut self, id: &str, label: &str, actor: bool) -> Result<(), FlowError> {
        if id.trim().is_empty() {
            return Err(FlowError::InvalidInput("participant id must not be empty".into()));
        }
        if self.has_participant(id) {
            return Err(FlowError::Duplicate(format!("participant '{id}'")));
        }
        self.participants.push(Participant {
            id: id.to_string(),
            label: label.to_string(),
            actor,
        });
        Ok(())
    }

    /// Add a message. Endpoints are auto-created as participants if missing
    /// (common in quick authoring); returns the new message index.
    pub fn add_message(
        &mut self,
        from: &str,
        to: &str,
        label: &str,
        kind: MessageKind,
    ) -> Result<usize, FlowError> {
        if from.trim().is_empty() || to.trim().is_empty() {
            return Err(FlowError::InvalidInput("message endpoints must not be empty".into()));
        }
        if !self.has_participant(from) {
            self.add_participant(from, from, false)?;
        }
        if !self.has_participant(to) {
            self.add_participant(to, to, false)?;
        }
        self.messages.push(Message {
            from: from.to_string(),
            to: to.to_string(),
            label: label.to_string(),
            kind,
        });
        Ok(self.messages.len() - 1)
    }

    /// Remove the message at `index`.
    pub fn remove_message(&mut self, index: usize) -> Result<(), FlowError> {
        if index >= self.messages.len() {
            return Err(FlowError::NotFound(format!("message index {index}")));
        }
        self.messages.remove(index);
        Ok(())
    }

    /// Column index of a participant id.
    pub fn col_of(&self, id: &str) -> Option<usize> {
        self.participants.iter().position(|p| p.id == id)
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_and_autocreate() {
        let mut s = Sequence::new();
        s.add_participant("u", "User", true).unwrap();
        let i = s.add_message("u", "api", "GET /x", MessageKind::Sync).unwrap();
        assert_eq!(i, 0);
        // "api" auto-created.
        assert!(s.has_participant("api"));
        assert_eq!(s.participants.len(), 2);
        s.add_message("api", "u", "200 OK", MessageKind::Return).unwrap();
        assert_eq!(s.messages.len(), 2);
    }

    #[test]
    fn rejects_duplicate_and_empty() {
        let mut s = Sequence::new();
        s.add_participant("a", "A", false).unwrap();
        assert!(s.add_participant("a", "A2", false).is_err());
        assert!(s.add_participant("", "x", false).is_err());
    }

    #[test]
    fn remove_message_bounds() {
        let mut s = Sequence::new();
        s.add_message("a", "b", "m", MessageKind::Sync).unwrap();
        assert!(s.remove_message(5).is_err());
        assert!(s.remove_message(0).is_ok());
        assert!(s.messages.is_empty());
    }
}
