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

    let (text, added) = merge(None, &q).unwrap();
    assert_eq!(added, 2);

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

    let (text, added) = merge(Some(existing), &q).unwrap();
    assert_eq!(added, 1, "only the new question");

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
    let (first, _) = merge(None, &q).unwrap();
    let (second, added) = merge(Some(&first), &q).unwrap();

    assert_eq!(added, 0);
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
    let (text, _) = merge(Some(existing), &q).unwrap();

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

    let (text, added) = merge(None, &q).unwrap();
    assert_eq!(added, 2);

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

    let (text, _) = merge(None, &q).unwrap();

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

#[test]
fn a_file_that_is_not_an_array_of_tables_is_an_error_not_a_silent_overwrite() {
    let broken = "question = 3\n";
    let q = questionnaire(vec![("Violence", "v", "Any violence?")]);

    let err = merge(Some(broken), &q).unwrap_err();
    assert!(
        format!("{err}").contains("array of tables"),
        "unexpected error: {err}"
    );
}
