use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SourceMaster {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSourceMasterRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSourceMasterRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Content {
    pub id: String,
    pub title: String,
    pub source: Option<String>,
    pub source_master_id: Option<String>,
    pub source_url: Option<String>,
    pub is_translating: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Sentence {
    pub id: String,
    pub content_id: String,
    pub sentence_index: i64,
    pub english_text: String,
    pub japanese_text: Option<String>,
    pub text_completed: bool,
    pub speech_completed: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateContentRequest {
    pub title: String,
    pub source_master_id: Option<String>,
    pub source_url: Option<String>,
    pub english_text: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateContentRequest {
    pub title: String,
    pub source_master_id: Option<String>,
    pub source_url: Option<String>,
    pub english_text: String,
}

#[derive(Debug, Serialize)]
pub struct ContentWithSentences {
    pub id: String,
    pub title: String,
    pub source: Option<String>,
    pub source_master_id: Option<String>,
    pub source_url: Option<String>,
    pub is_translating: bool,
    pub created_at: String,
    pub sentences: Vec<Sentence>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSentenceRequest {
    pub english_text: Option<String>,
    pub japanese_text: Option<String>,
    pub text_completed: Option<bool>,
    pub speech_completed: Option<bool>,
}

#[derive(Deserialize)]
pub struct ForceQuery {
    #[serde(default)]
    pub force: bool,
}

// ピリオドで終わる省略語（これに続く "." は文末ではない）
const ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "vs", "etc", "inc", "ltd", "corp",
    "dept", "est", "approx", "e.g", "i.e", "fig", "vol", "no", "pp", "ed",
    "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    "u.s", "u.k", "u.n",
];

fn is_abbreviation(word: &str) -> bool {
    let lower = word.to_lowercase();
    // 1文字（頭文字）は省略語とみなす
    if lower.len() == 1 && lower.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) {
        return true;
    }
    ABBREVIATIONS.contains(&lower.as_str())
}

pub fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        current.push(ch);

        if matches!(ch, '.' | '!' | '?') {
            // ピリオドの場合のみ省略語チェック
            if ch == '.' {
                // current から直前の単語を取り出す
                let before_dot = current[..current.len() - 1].trim_end();
                let last_word = before_dot
                    .split(|c: char| !c.is_alphanumeric() && c != '.')
                    .last()
                    .unwrap_or("");

                if is_abbreviation(last_word) {
                    i += 1;
                    continue;
                }

                // 次の文字が小文字なら文末ではない（ヒューリスティック）
                let next_non_space = chars[i + 1..]
                    .iter()
                    .find(|&&c| c != ' ' && c != '\t');
                if let Some(&next_ch) = next_non_space {
                    if next_ch.is_lowercase() {
                        i += 1;
                        continue;
                    }
                }
            }

            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current.clear();
        }

        i += 1;
    }

    let remaining = current.trim().to_string();
    if !remaining.is_empty() {
        sentences.push(remaining);
    }

    sentences
}
