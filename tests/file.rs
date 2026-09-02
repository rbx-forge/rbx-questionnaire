//! Pulling must never change an answer somebody wrote.

use rbx_questionnaire::config::Config;
use rbx_questionnaire::file::merge;
use rbx_questionnaire::model::{Opt, Question, Questionnaire, Section};

fn yes_no(id: &str, text: &str) -> Question {
    Question {
        id: id.to_string(),
        text: text.to_string(),
        question_type: "SingleSelect".to_string(),
        options: vec![
            Opt {
                id: format!("{id}-yes"),
                text: "Yes".to_string(),
                child_questions: Vec::new(),
            },
            Opt {
                id: format!("{id}-no"),
                text: "No".to_string(),
                child_questions: Vec::new(),
            },
        ],
    }
}

fn questionnaire(questions: Vec<(&str, &str, &str)>) -> Questionnaire {
    Questionnaire {
        id: "q1".to_string(),
        sections: questions
            .into_iter()
            .map(|(topic, id, text)| Section {
                id: format!("{topic}-s"),
                name: topic.to_string(),
                questions: vec![yes_no(id, text)],
            })
            .collect(),
    }
}

#[test]
fn a_first_pull_writes_every_question_unanswered() {
    let q = questionnaire(vec![
        ("Violence", "v", "Any violence?"),
        ("Blood", "b", "Any blood?"),
    ]);

    let (text, added) = merge(None, &q, None).unwrap();
    assert_eq!(added.added, 2);

    let config: Config = toml::from_str(&text).expect("the output must parse back");
    assert_eq!(config.questions.len(), 2);
    assert!(config.questions.iter().all(|q| q.answer.is_empty()));
    assert_eq!(config.get("v").unwrap().topic, "Violence");

    // The question itself is in the file, so it can be reviewed without
    // calling Roblox.
    assert!(text.contains("# Any violence?"), "{text}");
    assert!(text.contains("# options: Yes, No"), "{text}");
}

/// The rule that matters: an answer already filed is a statement somebody made,
/// and a pull is not allowed to restate it.
#[test]
fn a_pull_never_rewrites_an_existing_answer() {
    let existing = r#"
[[question]]
id = "v"
topic = "Violence"
answer = "yes"   # we do have some
"#;

    let q = questionnaire(vec![
        ("Violence", "v", "Any violence?"),
        ("Blood", "b", "Any blood?"),
    ]);

    let (text, added) = merge(Some(existing), &q, None).unwrap();
    assert_eq!(added.added, 1, "only the new question");

    let config: Config = toml::from_str(&text).unwrap();
    assert_eq!(config.get("v").unwrap().answer, "yes");
    assert_eq!(config.get("b").unwrap().answer, "");

    // And the note beside it survives, which is the reason for editing the
    // document rather than re-serializing it.
    assert!(text.contains("# we do have some"), "{text}");
}

#[test]
fn a_pull_with_nothing_new_adds_nothing() {
    let q = questionnaire(vec![("Violence", "v", "Any violence?")]);
    let (first, _) = merge(None, &q, None).unwrap();
    let (second, added) = merge(Some(&first), &q, None).unwrap();

    assert_eq!(added.added, 0);
    assert_eq!(first, second, "a no-op pull must not touch the file");
}

/// A question Roblox dropped stays in the file. Removing it would delete a
/// declaration; reporting it is `check`'s job.
#[test]
fn a_pull_never_removes_a_question() {
    let existing = r#"
[[question]]
id = "gone"
topic = "Removed"
answer = "no"
"#;

    let q = questionnaire(vec![("Violence", "v", "Any violence?")]);
    let (text, _) = merge(Some(existing), &q, None).unwrap();

    let config: Config = toml::from_str(&text).unwrap();
    assert!(config.get("gone").is_some(), "{text}");
    assert!(config.get("v").is_some(), "{text}");
}

/// Follow-ups are seeded too, so the file is complete whichever way its answers
/// go rather than growing a question at a time as answers change.
#[test]
fn follow_up_questions_are_written_too() {
    let mut violence = yes_no("v", "Any violence?");
    violence.options[0].child_questions = vec![yes_no("v-realistic", "Is it realistic?")];

    let q = Questionnaire {
        id: "q1".to_string(),
        sections: vec![Section {
            id: "s".to_string(),
            name: "Violence".to_string(),
            questions: vec![violence],
        }],
    };

    let (text, added) = merge(None, &q, None).unwrap();
    assert_eq!(added.added, 2);

    let config: Config = toml::from_str(&text).unwrap();
    assert_eq!(config.get("v-realistic").unwrap().topic, "Violence");
}

/// Found by running this against a real universe, where `pull` wrote a file
/// that nothing could read back.
///
/// Roblox's question text is markdown and runs to a dozen lines: a definition,
/// a bulleted list of what not to count, sometimes a link. Prefixing only the
/// first line left line two to be parsed as TOML.
#[test]
fn a_multi_line_question_is_commented_on_every_line() {
    let mut question = yes_no("v", "Any violence?");
    question.text = "#### Does this experience contain violence?\n\
                     **Violence**: Depictions of the act and/or the consequence.\n\
                     **Do NOT measure**:\n\
                     * Character death due to accidents.\n\
                     * Nonviolent adversarial games."
        .to_string();

    let q = Questionnaire {
        id: "q1".to_string(),
        sections: vec![Section {
            id: "s".to_string(),
            name: "Violence".to_string(),
            questions: vec![question],
        }],
    };

    let (text, _) = merge(None, &q, None).unwrap();

    // The whole point: it must survive a round trip.
    let config: Config = toml::from_str(&text).expect("the output must parse back");
    assert_eq!(config.questions.len(), 1);

    for line in text.lines() {
        let line = line.trim();
        assert!(
            !line.starts_with("**") && !line.starts_with('*'),
            "a markdown line escaped the comment: {line}"
        );
    }
    assert!(
        text.contains("# * Character death due to accidents."),
        "{text}"
    );
}

/// The shortcut for a questionnaire that really is "no" throughout, and the
/// reason it is a flag rather than a default: the answers land in the file,
/// where they are committed and can be read and disagreed with.
#[test]
fn fill_writes_the_answer_into_the_file_rather_than_assuming_it() {
    let q = questionnaire(vec![
        ("Violence", "v", "Any violence?"),
        ("Blood", "b", "Any blood?"),
    ]);

    let (text, merged) = merge(None, &q, Some("no")).unwrap();

    assert_eq!(merged.added, 2);
    assert_eq!(merged.filled, 2);

    let config: Config = toml::from_str(&text).unwrap();
    assert_eq!(config.get("v").unwrap().answer, "No");
    assert_eq!(config.get("b").unwrap().answer, "No");
}

/// Fill is not allowed to restate an answer somebody already wrote, which is the
/// same rule that governs the rest of a pull.
#[test]
fn fill_never_overwrites_an_answer_already_there() {
    let existing = r#"
[[question]]
id = "v"
topic = "Violence"
answer = "yes"
"#;

    let q = questionnaire(vec![("Violence", "v", "Any violence?")]);
    let (text, merged) = merge(Some(existing), &q, Some("no")).unwrap();

    assert_eq!(merged.filled, 0);
    let config: Config = toml::from_str(&text).unwrap();
    assert_eq!(config.get("v").unwrap().answer, "yes");
}

/// A blank already in the file is filled, so a second `pull --fill` finishes
/// what a first one could not reach.
#[test]
fn fill_completes_blanks_that_were_already_in_the_file() {
    let q = questionnaire(vec![("Violence", "v", "Any violence?")]);

    let (first, _) = merge(None, &q, None).unwrap();
    let (second, merged) = merge(Some(&first), &q, Some("no")).unwrap();

    assert_eq!(merged.added, 0);
    assert_eq!(merged.filled, 1);
    let config: Config = toml::from_str(&second).unwrap();
    assert_eq!(config.get("v").unwrap().answer, "No");
}

/// A question with no matching option is left blank and named, not forced. The
/// real follow-ups offer Mild/Moderate, and "no" is not one of them.
#[test]
fn fill_leaves_a_question_it_cannot_answer_blank_and_says_which() {
    let mut violence = yes_no("v", "Any violence?");
    violence.options[0].child_questions = vec![Question {
        id: "v-level".to_string(),
        text: "How intense?".to_string(),
        question_type: "SingleSelect".to_string(),
        options: vec![
            Opt {
                id: "mild".to_string(),
                text: "Mild".to_string(),
                child_questions: Vec::new(),
            },
            Opt {
                id: "moderate".to_string(),
                text: "Moderate".to_string(),
                child_questions: Vec::new(),
            },
        ],
    }];

    let q = Questionnaire {
        id: "q1".to_string(),
        sections: vec![Section {
            id: "s".to_string(),
            name: "Violence".to_string(),
            questions: vec![violence],
        }],
    };

    let (text, merged) = merge(None, &q, Some("no")).unwrap();

    assert_eq!(merged.filled, 1, "only the yes/no question");
    assert_eq!(merged.unfilled, vec!["Violence".to_string()]);

    let config: Config = toml::from_str(&text).unwrap();
    assert_eq!(config.get("v").unwrap().answer, "No");
    assert_eq!(config.get("v-level").unwrap().answer, "");
}

#[test]
fn a_file_that_is_not_an_array_of_tables_is_an_error_not_a_silent_overwrite() {
    let broken = "question = 3\n";
    let q = questionnaire(vec![("Violence", "v", "Any violence?")]);

    let err = merge(Some(broken), &q, None).unwrap_err();
    assert!(
        format!("{err}").contains("array of tables"),
        "unexpected error: {err}"
    );
}
