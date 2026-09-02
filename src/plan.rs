//! Matching what you declared against what Roblox is asking.
//!
//! This is the whole tool, minus the network. It is pure: a questionnaire in, a
//! config in, a set of answers or a set of reasons why not out. That is what
//! makes it testable, and it is the part worth being sure about, because the
//! output of this file is a declaration filed under your name.
//!
//! The subtlety it exists for: which questions are required depends on the
//! answers. Every option can carry follow-up questions that only exist once
//! that option is chosen. Answering "no" throughout reveals nothing, which is
//! why a tool that only ever answers "no" appears to work and then falls over
//! the first time somebody declares a "yes".

use crate::config::Config;
use crate::model::{Answer, Question, Questionnaire};

#[derive(Debug, Default)]
pub struct Plan {
    /// Ready to submit, in the order the questions are asked.
    pub answers: Vec<Answer>,
    /// Why this cannot be submitted yet.
    pub problems: Vec<Problem>,
    /// The questions these answers actually reach, for display.
    pub reached: Vec<Reached>,
}

impl Plan {
    /// Not "no problems": a question Roblox has dropped is worth telling
    /// somebody about, and is no reason to refuse a declaration that is
    /// otherwise complete.
    pub fn is_submittable(&self) -> bool {
        !self.answers.is_empty() && !self.problems.iter().any(Problem::blocks_submission)
    }

    /// The problems that stop a submission, as opposed to the ones that are
    /// only worth mentioning.
    pub fn blocking(&self) -> impl Iterator<Item = &Problem> {
        self.problems.iter().filter(|p| p.blocks_submission())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reached {
    pub id: String,
    pub topic: String,
    pub text: String,
    /// The option text that was chosen.
    pub answer: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// Roblox asks it, the config does not answer it.
    ///
    /// Never defaulted. A content declaration nobody wrote is not a
    /// declaration, and this tool will not make one up.
    Unanswered {
        id: String,
        topic: String,
        text: String,
        options: Vec<String>,
    },
    /// The config answers it with something the question does not offer.
    UnknownAnswer {
        id: String,
        topic: String,
        answer: String,
        options: Vec<String>,
    },
    /// The config answers a question this questionnaire no longer contains.
    ///
    /// Not fatal to a submission on its own, but reported: it usually means
    /// Roblox reworked the questionnaire and the file needs a fresh `pull`.
    Stale { id: String, topic: String },
}

impl Problem {
    pub fn blocks_submission(&self) -> bool {
        !matches!(self, Problem::Stale { .. })
    }
}

/// Work out what to submit.
pub fn plan(questionnaire: &Questionnaire, config: &Config) -> Plan {
    let mut plan = Plan::default();

    for question in questionnaire.top_level_questions() {
        walk(questionnaire, question, config, &mut plan);
    }

    // A declared answer to a question that no longer exists anywhere in the
    // tree, reached or not. An unreached follow-up is deliberately not stale:
    // keeping its answer is what lets somebody flip a "yes" back and forth
    // without losing what they had written underneath.
    let live: Vec<&str> = questionnaire
        .all_questions()
        .iter()
        .map(|q| q.id.as_str())
        .collect();

    for id in config.extra_ids(&live) {
        let topic = config.get(id).map(|d| d.topic.clone()).unwrap_or_default();
        plan.problems.push(Problem::Stale {
            id: id.to_string(),
            topic,
        });
    }

    plan
}

fn walk(root: &Questionnaire, question: &Question, config: &Config, plan: &mut Plan) {
    let topic = root
        .section_of(&question.id)
        .unwrap_or_default()
        .to_string();

    let Some(declared) = config.get(&question.id) else {
        // Stop here rather than guessing. Which follow-ups this question
        // reveals depends on the answer, so without one there is nothing
        // further to say about the branch underneath it.
        plan.problems.push(Problem::Unanswered {
            id: question.id.clone(),
            topic,
            text: question.text.clone(),
            options: question.option_texts(),
        });
        return;
    };

    // By text first, because that is what a person writes. By id as the escape
    // hatch for a question whose options are not words.
    let chosen = question
        .option_by_text(&declared.answer)
        .or_else(|| question.option_by_id(&declared.answer));

    let Some(option) = chosen else {
        plan.problems.push(Problem::UnknownAnswer {
            id: question.id.clone(),
            topic,
            answer: declared.answer.clone(),
            options: question.option_texts(),
        });
        return;
    };

    plan.answers.push(Answer::new(&question.id, &option.id));
    plan.reached.push(Reached {
        id: question.id.clone(),
        topic,
        text: question.text.clone(),
        answer: option.text.clone(),
    });

    for child in &option.child_questions {
        walk(root, child, config, plan);
    }
}
