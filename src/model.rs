//! The shapes Roblox's questionnaire API returns.
//!
//! None of this is in Roblox's published Open Cloud specification, so nothing
//! validates it but the live endpoint. Every field that is not needed to answer
//! a question is `#[serde(default)]`, so a field Roblox adds or renames costs a
//! blank rather than a failed parse.

use serde::{Deserialize, Serialize};

/// `GET /questionnaires/{universeId}/latest`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Latest {
    pub questionnaire_id: String,
}

/// `GET /questionnaires/{id}` returns the questionnaire under a wrapper.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wrapper {
    pub questionnaire: Questionnaire,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Questionnaire {
    pub id: String,
    #[serde(default)]
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub questions: Vec<Question>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default, rename = "type")]
    pub question_type: String,
    #[serde(default)]
    pub options: Vec<Opt>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opt {
    pub id: String,
    #[serde(default)]
    pub text: String,
    /// Questions that only exist once this option is chosen.
    ///
    /// This is why answering "no" to everything appears to work: "no" reveals
    /// nothing, so the follow-ups never have to be answered. Declaring a "yes"
    /// makes them appear, and they are as required as the rest.
    #[serde(default)]
    pub child_questions: Vec<Question>,
}

/// One answer, in the shape the API wants it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub question_id: String,
    /// A JSON-encoded string: the option id, with its quotes, inside a string.
    /// That is the API's own encoding, not a mistake here.
    pub value: String,
}

impl Answer {
    pub fn new(question_id: &str, option_id: &str) -> Self {
        Self {
            question_id: question_id.to_string(),
            // `serde_json::to_string` rather than `format!("\"{id}\"")`: an
            // option id with a quote or a backslash in it would otherwise
            // produce a body that is not valid JSON.
            value: serde_json::to_string(option_id).expect("a string always serializes"),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateResponse {
    #[serde(default)]
    pub is_valid: bool,
    #[serde(default)]
    pub failures: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResponse {
    #[serde(default)]
    pub age_recommendation_details: Option<serde_json::Value>,
    #[serde(default)]
    pub disable_submit_button: bool,
}

impl PreviewResponse {
    /// The age rating these answers would produce, if the response says.
    ///
    /// Dug out rather than typed, because the shape of this object is not
    /// documented anywhere and a wrong struct would turn "we could not read the
    /// rating" into "the request failed".
    pub fn age_rating(&self) -> Option<String> {
        let details = self.age_recommendation_details.as_ref()?;

        for key in ["displayName", "ageRecommendation", "name", "displayText"] {
            if let Some(found) = find_string(details, key) {
                return Some(found);
            }
        }
        None
    }
}

/// Depth-first search for the first string value under `key`.
fn find_string(value: &serde_json::Value, key: &str) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(s)) = map.get(key) {
                return Some(s.clone());
            }
            map.values().find_map(|v| find_string(v, key))
        }
        serde_json::Value::Array(items) => items.iter().find_map(|v| find_string(v, key)),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitResponse {
    #[serde(default)]
    pub submission_id: String,
}

impl Questionnaire {
    /// Every question, follow-ups included, in the order they are asked.
    pub fn all_questions(&self) -> Vec<&Question> {
        let mut out = Vec::new();
        for section in &self.sections {
            for question in &section.questions {
                collect(question, &mut out);
            }
        }
        out
    }

    /// The questions asked up front, without the follow-ups any answer reveals.
    pub fn top_level_questions(&self) -> Vec<&Question> {
        self.sections
            .iter()
            .flat_map(|s| s.questions.iter())
            .collect()
    }

    /// The section a question belongs to, for a readable label.
    pub fn section_of(&self, question_id: &str) -> Option<&str> {
        self.sections
            .iter()
            .find(|s| {
                let mut found = Vec::new();
                for q in &s.questions {
                    collect(q, &mut found);
                }
                found.iter().any(|q| q.id == question_id)
            })
            .map(|s| s.name.as_str())
    }
}

fn collect<'a>(question: &'a Question, out: &mut Vec<&'a Question>) {
    out.push(question);
    for option in &question.options {
        for child in &option.child_questions {
            collect(child, out);
        }
    }
}

impl Question {
    /// The option whose text matches, ignoring case and surrounding space.
    pub fn option_by_text(&self, text: &str) -> Option<&Opt> {
        self.options
            .iter()
            .find(|o| o.text.trim().eq_ignore_ascii_case(text.trim()))
    }

    pub fn option_by_id(&self, id: &str) -> Option<&Opt> {
        self.options.iter().find(|o| o.id == id)
    }

    pub fn option_texts(&self) -> Vec<String> {
        self.options.iter().map(|o| o.text.clone()).collect()
    }
}
