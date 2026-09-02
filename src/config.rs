//! `rbxquestionnaire.toml`: what this experience declares about its content.
//!
//! The point of a file rather than a flag is that a content declaration to
//! Roblox is a statement about the game, and a statement about the game belongs
//! next to the game, in version control, where a person can read it and a
//! reviewer can disagree with it. A default answer is not a statement anybody
//! made.
//!
//! Each entry is keyed by Roblox's own question id. That is what stays stable
//! when a question is reworded, and it is what lets `check` tell a question
//! Roblox has added apart from one your file already answers.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const DEFAULT_FILE: &str = "rbxquestionnaire.toml";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default, rename = "question")]
    pub questions: Vec<Declared>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Declared {
    /// Roblox's question id. The identity, because text gets reworded.
    pub id: String,

    /// A human label, from the section the question sits in. Never matched on;
    /// it is here so the file can be read without calling Roblox.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub topic: String,

    /// The answer, matched against the option's text: usually `"yes"` or
    /// `"no"`, but whatever Roblox offers for a question that is not a
    /// yes/no.
    pub answer: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn get(&self, question_id: &str) -> Option<&Declared> {
        self.questions.iter().find(|q| q.id == question_id)
    }

    /// Ids declared here that the live questionnaire no longer asks.
    pub fn extra_ids<'a>(&'a self, live_ids: &[&str]) -> Vec<&'a str> {
        self.questions
            .iter()
            .map(|q| q.id.as_str())
            .filter(|id| !live_ids.contains(id))
            .collect()
    }
}
