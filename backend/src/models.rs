use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Content {
    pub id: String,
    pub title: String,
    pub source: Option<String>,
    pub summary: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Sentence {
    pub id: String,
    pub content_id: String,
    pub sentence_index: i64,
    pub english_text: String,
    pub japanese_text: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateContentRequest {
    pub title: String,
    pub source: Option<String>,
    pub english_text: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateContentRequest {
    pub title: String,
    pub source: Option<String>,
    pub english_text: String,
}

#[derive(Debug, Serialize)]
pub struct ContentWithSentences {
    pub id: String,
    pub title: String,
    pub source: Option<String>,
    pub summary: Option<String>,
    pub created_at: String,
    pub sentences: Vec<Sentence>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSentenceRequest {
    pub japanese_text: Option<String>,
}

#[derive(Deserialize)]
pub struct ForceQuery {
    #[serde(default)]
    pub force: bool,
}

pub fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current.clear();
        }
    }

    let remaining = current.trim().to_string();
    if !remaining.is_empty() {
        sentences.push(remaining);
    }

    sentences
}
