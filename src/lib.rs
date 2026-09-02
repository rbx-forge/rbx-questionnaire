//! Declare, review and submit a Roblox experience's content questionnaire.
//!
//! Roblox turns the answers into the experience's age rating, which makes this
//! a statement about the game rather than a deployment setting. So the answers
//! live in a file you commit, nothing is ever defaulted, and an unanswered
//! question stops a submission instead of being guessed at.
//!
//! The part worth being sure about is [`plan`], which is pure: a questionnaire
//! and a config in, a set of answers or a set of reasons why not out.

pub mod api;
pub mod config;
pub mod env;
pub mod file;
pub mod model;
pub mod plan;

pub use plan::{plan, Plan, Problem, Reached};
