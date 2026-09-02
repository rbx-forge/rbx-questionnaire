//! What gets declared, and what refuses to be.
//!
//! Built from constructed questionnaires rather than recorded responses: these
//! endpoints are undocumented and a recorded body would go stale silently,
//! while the rules being tested here are ours and do not.

use rbx_questionnaire::config::{Config, Declared};
use rbx_questionnaire::model::{Opt, Question, Questionnaire, Section};
use rbx_questionnaire::plan::{plan, Problem};

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

fn section(name: &str, questions: Vec<Question>) -> Section {
    Section {
        id: format!("{name}-section"),
        name: name.to_string(),
        questions,
    }
}

fn questionnaire(sections: Vec<Section>) -> Questionnaire {
    Questionnaire {
        id: "q1".to_string(),
        sections,
    }
}

fn config(pairs: &[(&str, &str)]) -> Config {
    Config {
        questions: pairs
            .iter()
            .map(|(id, answer)| Declared {
                id: id.to_string(),
                topic: String::new(),
                answer: answer.to_string(),
            })
            .collect(),
    }
}

#[test]
fn a_fully_answered_questionnaire_is_submittable() {
    let q = questionnaire(vec![
        section("Violence", vec![yes_no("v", "Any violence?")]),
        section("Blood", vec![yes_no("b", "Any blood?")]),
    ]);

    let p = plan(&q, &config(&[("v", "no"), ("b", "yes")]));

    assert!(p.problems.is_empty(), "{:?}", p.problems);
    assert!(p.is_submittable());
    assert_eq!(p.answers.len(), 2);
    assert_eq!(p.answers[0].question_id, "v");
    assert_eq!(p.answers[0].value, "\"v-no\"");
    assert_eq!(p.answers[1].value, "\"b-yes\"");
}

/// The rule the whole tool exists for. A content declaration nobody wrote is
/// not a declaration, so a missing answer is never filled in.
#[test]
fn a_missing_answer_is_never_defaulted() {
    let q = questionnaire(vec![section(
        "Violence",
        vec![yes_no("v", "Any violence?")],
    )]);

    let p = plan(&q, &Config::default());

    assert!(p.answers.is_empty());
    assert!(!p.is_submittable());
    match &p.problems[0] {
        Problem::Unanswered {
            id, topic, options, ..
        } => {
            assert_eq!(id, "v");
            assert_eq!(topic, "Violence");
            assert_eq!(options, &["Yes".to_string(), "No".to_string()]);
        }
        other => panic!("expected Unanswered, got {other:?}"),
    }
}

#[test]
fn an_answer_the_question_does_not_offer_is_refused() {
    let q = questionnaire(vec![section(
        "Violence",
        vec![yes_no("v", "Any violence?")],
    )]);

    let p = plan(&q, &config(&[("v", "maybe")]));

    assert!(!p.is_submittable());
    match &p.problems[0] {
        Problem::UnknownAnswer {
            answer, options, ..
        } => {
            assert_eq!(answer, "maybe");
            assert_eq!(options.len(), 2);
        }
        other => panic!("expected UnknownAnswer, got {other:?}"),
    }
}

#[test]
fn answers_match_case_insensitively_and_ignore_surrounding_space() {
    let q = questionnaire(vec![section(
        "Violence",
        vec![yes_no("v", "Any violence?")],
    )]);

    for written in ["no", "No", "NO", "  no  "] {
        let p = plan(&q, &config(&[("v", written)]));
        assert!(
            p.problems.is_empty(),
            "{written:?} should match: {:?}",
            p.problems
        );
    }
}

/// The escape hatch, for a question whose options are not words a person would
/// want to retype.
#[test]
fn an_option_id_works_as_an_answer() {
    let q = questionnaire(vec![section(
        "Violence",
        vec![yes_no("v", "Any violence?")],
    )]);

    let p = plan(&q, &config(&[("v", "v-yes")]));

    assert!(p.problems.is_empty(), "{:?}", p.problems);
    assert_eq!(p.answers[0].value, "\"v-yes\"");
}

/// Roblox's options are not the words you would expect. The real violence
/// question offers "No" and "Yes (Contains Violence)", so an exact match would
/// make somebody retype that to declare it.
#[test]
fn an_unambiguous_prefix_resolves_to_the_option_it_fits() {
    let q = questionnaire(vec![section(
        "Violence",
        vec![Question {
            id: "v".to_string(),
            text: "Any violence?".to_string(),
            question_type: "SingleSelect".to_string(),
            options: vec![
                Opt {
                    id: "v-no".to_string(),
                    text: "No".to_string(),
                    child_questions: Vec::new(),
                },
                Opt {
                    id: "v-yes".to_string(),
                    text: "Yes (Contains Violence)".to_string(),
                    child_questions: Vec::new(),
                },
            ],
        }],
    )]);

    let p = plan(&q, &config(&[("v", "yes")]));

    assert!(p.problems.is_empty(), "{:?}", p.problems);
    assert_eq!(p.answers[0].value, "\"v-yes\"");
    assert_eq!(p.reached[0].answer, "Yes (Contains Violence)");
}

/// But a prefix that fits two options is refused rather than resolved to the
/// first: choosing between them would be choosing what the game declares.
#[test]
fn a_prefix_that_fits_two_options_is_refused() {
    let q = questionnaire(vec![section(
        "Violence",
        vec![Question {
            id: "v".to_string(),
            text: "How much?".to_string(),
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
        }],
    )]);

    let p = plan(&q, &config(&[("v", "M")]));

    assert!(!p.is_submittable());
    match &p.problems[0] {
        Problem::AmbiguousAnswer { matches, .. } => assert_eq!(matches.len(), 2),
        other => panic!("expected AmbiguousAnswer, got {other:?}"),
    }

    // Enough of it to tell them apart resolves.
    let p = plan(&q, &config(&[("v", "Mod")]));
    assert!(p.problems.is_empty(), "{:?}", p.problems);
    assert_eq!(p.answers[0].value, "\"moderate\"");
}

/// `pull` writes `answer = ""`, so a freshly pulled file is full of them. That
/// is unanswered, not wrongly answered, and the message has to say so.
#[test]
fn a_blank_answer_reads_as_unanswered_rather_than_wrong() {
    let q = questionnaire(vec![section(
        "Violence",
        vec![yes_no("v", "Any violence?")],
    )]);

    for blank in ["", "   "] {
        let p = plan(&q, &config(&[("v", blank)]));
        match &p.problems[0] {
            Problem::Unanswered { id, .. } => assert_eq!(id, "v"),
            other => panic!("{blank:?} should read as unanswered, got {other:?}"),
        }
    }
}

// ─── Follow-up questions ─────────────────────────────────────

/// The case a tool that only ever answers "no" never meets: saying yes reveals
/// questions that are as required as the rest.
#[test]
fn saying_yes_reveals_follow_ups_that_must_also_be_answered() {
    let mut violence = yes_no("v", "Any violence?");
    violence.options[0].child_questions = vec![yes_no("v-realistic", "Is it realistic?")];

    let q = questionnaire(vec![section("Violence", vec![violence])]);

    // Yes, and nothing said about the follow-up.
    let p = plan(&q, &config(&[("v", "yes")]));
    assert!(!p.is_submittable());
    assert_eq!(p.problems.len(), 1);
    match &p.problems[0] {
        Problem::Unanswered { id, .. } => assert_eq!(id, "v-realistic"),
        other => panic!("expected the follow-up unanswered, got {other:?}"),
    }

    // Yes, and the follow-up answered.
    let p = plan(&q, &config(&[("v", "yes"), ("v-realistic", "no")]));
    assert!(p.problems.is_empty(), "{:?}", p.problems);
    assert_eq!(p.answers.len(), 2);
}

/// And the converse, which is what makes "no" look easy: an unreached follow-up
/// is not required, and must not be reported or submitted.
#[test]
fn a_follow_up_behind_an_unchosen_option_is_not_required() {
    let mut violence = yes_no("v", "Any violence?");
    violence.options[0].child_questions = vec![yes_no("v-realistic", "Is it realistic?")];

    let q = questionnaire(vec![section("Violence", vec![violence])]);
    let p = plan(&q, &config(&[("v", "no")]));

    assert!(p.problems.is_empty(), "{:?}", p.problems);
    assert_eq!(p.answers.len(), 1, "the follow-up must not be submitted");
    assert_eq!(p.reached.len(), 1);
}

/// Keeping the answer to an unreached follow-up is deliberate: it is what lets
/// somebody flip a yes back and forth without losing what they wrote under it.
#[test]
fn an_unreached_follow_up_keeps_its_answer_without_complaint() {
    let mut violence = yes_no("v", "Any violence?");
    violence.options[0].child_questions = vec![yes_no("v-realistic", "Is it realistic?")];

    let q = questionnaire(vec![section("Violence", vec![violence])]);
    let p = plan(&q, &config(&[("v", "no"), ("v-realistic", "yes")]));

    assert!(p.problems.is_empty(), "{:?}", p.problems);
    assert_eq!(p.answers.len(), 1);
}

#[test]
fn follow_ups_nest_more_than_one_deep() {
    let mut inner = yes_no("v-realistic", "Is it realistic?");
    inner.options[0].child_questions = vec![yes_no("v-gore", "Is there gore?")];

    let mut violence = yes_no("v", "Any violence?");
    violence.options[0].child_questions = vec![inner];

    let q = questionnaire(vec![section("Violence", vec![violence])]);

    let p = plan(&q, &config(&[("v", "yes"), ("v-realistic", "yes")]));
    match &p.problems[0] {
        Problem::Unanswered { id, .. } => assert_eq!(id, "v-gore"),
        other => panic!("expected the nested follow-up, got {other:?}"),
    }

    let p = plan(
        &q,
        &config(&[("v", "yes"), ("v-realistic", "yes"), ("v-gore", "no")]),
    );
    assert!(p.problems.is_empty(), "{:?}", p.problems);
    assert_eq!(p.answers.len(), 3);
}

/// Without an answer there is no way to know which branch is required, so the
/// walk stops rather than reporting follow-ups that may not apply.
#[test]
fn an_unanswered_question_does_not_report_its_follow_ups_too() {
    let mut violence = yes_no("v", "Any violence?");
    violence.options[0].child_questions = vec![yes_no("v-realistic", "Is it realistic?")];

    let q = questionnaire(vec![section("Violence", vec![violence])]);
    let p = plan(&q, &Config::default());

    assert_eq!(
        p.problems.len(),
        1,
        "one problem, not two: {:?}",
        p.problems
    );
}

// ─── Drift ───────────────────────────────────────────────────

/// Roblox adding a question is the failure this file format is designed to
/// surface: it shows up as unanswered rather than being silently defaulted,
/// which is what a hardcoded table of answers would have done.
#[test]
fn a_question_roblox_added_shows_up_as_unanswered() {
    let q = questionnaire(vec![
        section("Violence", vec![yes_no("v", "Any violence?")]),
        section("AI Interaction", vec![yes_no("ai", "Any AI?")]),
    ]);

    let p = plan(&q, &config(&[("v", "no")]));

    assert!(!p.is_submittable());
    match &p.problems[0] {
        Problem::Unanswered { topic, .. } => assert_eq!(topic, "AI Interaction"),
        other => panic!("expected AI Interaction unanswered, got {other:?}"),
    }
}

/// And a question Roblox removed is reported without blocking anything: the
/// declaration is still complete, the file merely has an extra line in it.
#[test]
fn a_question_roblox_removed_is_reported_but_does_not_block() {
    let q = questionnaire(vec![section(
        "Violence",
        vec![yes_no("v", "Any violence?")],
    )]);

    let p = plan(&q, &config(&[("v", "no"), ("gone", "no")]));

    assert!(p.is_submittable(), "{:?}", p.problems);
    assert_eq!(p.problems.len(), 1);
    assert!(matches!(p.problems[0], Problem::Stale { .. }));
    assert!(!p.problems[0].blocks_submission());
}

#[test]
fn an_empty_questionnaire_is_not_submittable() {
    let p = plan(&questionnaire(Vec::new()), &Config::default());

    assert!(p.problems.is_empty());
    assert!(!p.is_submittable(), "there is nothing to declare");
}

/// The API wants the option id as a JSON string inside a string. An id with a
/// quote in it would otherwise produce a body that is not valid JSON.
#[test]
fn an_option_id_with_a_quote_is_encoded_not_interpolated() {
    let q = questionnaire(vec![section(
        "Odd",
        vec![Question {
            id: "odd".to_string(),
            text: "?".to_string(),
            question_type: "SingleSelect".to_string(),
            options: vec![Opt {
                id: "we\"ird".to_string(),
                text: "No".to_string(),
                child_questions: Vec::new(),
            }],
        }],
    )]);

    let p = plan(&q, &config(&[("odd", "no")]));

    assert_eq!(p.answers[0].value, r#""we\"ird""#);
    serde_json::from_str::<String>(&p.answers[0].value).expect("must be valid JSON");
}
