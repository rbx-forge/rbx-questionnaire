//! Writing `rbxquestionnaire.toml` without trampling what is already in it.
//!
//! `toml_edit` rather than serializing the whole struct back out, for one
//! reason: this file holds answers a person wrote and, usually, the note
//! explaining why. Re-serializing would reformat their file and drop every
//! comment, which is a bad trade for adding one question Roblox introduced last
//! week.
//!
//! So a pull only ever *appends*. It never rewrites an answer and never removes
//! an entry, because either would change a declaration filed under somebody's
//! name without them saying so. Entries Roblox no longer asks are reported, not
//! deleted.

use anyhow::{Context, Result};
use toml_edit::{value, ArrayOfTables, DocumentMut, Item, Table};

use crate::model::Questionnaire;

const HEADER: &str = "\
# What this experience declares about its content.
#
# Roblox turns these answers into the experience's age rating, so this file is a
# statement about the game rather than a deployment setting. It is answered by
# hand, on purpose: nothing here is defaulted, and an unanswered question stops
# a submission rather than being guessed at.
#
# `id` is Roblox's own question id, which is what stays stable when a question
# is reworded. `topic` is a label for people and is never matched on. `answer`
# is matched against the option text the question offers, usually yes or no.
#
# Run `rbx-questionnaire pull` after Roblox changes the questionnaire: it adds
# what is missing and touches nothing that is already here.

";

/// Add an entry for every question not already in `existing`.
///
/// Returns the new document and how many entries were added.
pub fn merge(existing: Option<&str>, questionnaire: &Questionnaire) -> Result<(String, usize)> {
    let mut doc: DocumentMut = match existing {
        Some(text) => text.parse().context("parsing the existing config")?,
        None => {
            let mut fresh = DocumentMut::new();
            fresh.decor_mut().set_prefix(HEADER);
            fresh
        }
    };

    let entry = doc
        .entry("question")
        .or_insert(Item::ArrayOfTables(ArrayOfTables::new()));

    let tables = entry
        .as_array_of_tables_mut()
        .context("`question` in this file is not an array of tables")?;

    let known: Vec<String> = tables
        .iter()
        .filter_map(|t| t.get("id").and_then(|v| v.as_str()).map(str::to_string))
        .collect();

    let mut added = 0;

    // Every question in the tree, follow-ups included, so the file is complete
    // whichever way its answers go. A follow-up nobody reaches costs three
    // lines; discovering a required question only at submission costs a round
    // trip and a confused user.
    for question in questionnaire.all_questions() {
        if known.iter().any(|id| id == &question.id) {
            continue;
        }

        let mut table = Table::new();
        table.insert("id", value(&question.id));
        table.insert(
            "topic",
            value(questionnaire.section_of(&question.id).unwrap_or_default()),
        );
        table.insert("answer", value(""));

        // The question itself, as a comment, so the file can be read and
        // reviewed without calling Roblox.
        let mut prefix = format!("\n{}", comment(&question.text));
        let options = question.option_texts();
        if !options.is_empty() {
            prefix.push_str(&comment(&format!("options: {}", options.join(", "))));
        }
        table.decor_mut().set_prefix(prefix);

        tables.push(table);
        added += 1;
    }

    Ok((doc.to_string(), added))
}

/// Comment out every line of `text`, not just the first.
///
/// Roblox's question text is markdown and runs to a dozen lines: a definition,
/// a bulleted list of what not to count, sometimes a link. Prefixing only the
/// first line produced a file that `pull` wrote happily and nothing could read
/// back, because line two was parsed as TOML.
fn comment(text: &str) -> String {
    let mut out = String::new();

    for line in text.trim().lines() {
        let line = line.trim_end();
        if line.is_empty() {
            out.push_str("#\n");
        } else {
            out.push_str("# ");
            out.push_str(line);
            out.push('\n');
        }
    }

    out
}
