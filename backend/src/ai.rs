use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

fn ollama_url() -> String {
    std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://ollama:11434".to_string())
}

fn ollama_model() -> String {
    std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:3b".to_string())
}

async fn generate(prompt: String, timeout_secs: u64) -> Result<String, String> {
    let client = Client::new();
    let req = OllamaRequest {
        model: ollama_model(),
        prompt,
        stream: false,
    };

    let res = client
        .post(format!("{}/api/generate", ollama_url()))
        .json(&req)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .send()
        .await
        .map_err(|e| format!("Ollamaに接続できません: {}", e))?;

    let body: OllamaResponse = res
        .json()
        .await
        .map_err(|e| format!("Ollamaのレスポンス解析エラー: {}", e))?;

    Ok(body.response.trim().to_string())
}

pub async fn translate_sentences(sentences: &[String]) -> Result<Vec<String>, String> {
    let mut results = Vec::new();
    for sentence in sentences {
        let prompt = format!(
            r#"あなたはプロの翻訳者です。以下の英文を自然な日本語口語に翻訳してください。

ルール：
- 直訳ではなく、日本語として自然な表現を使う
- 会話文・話し言葉の場合はそのニュアンスを保つ
- IT・テクノロジー用語はカタカナで（例: テザリング、ノートパソコン、スマホ）
- 日本語訳のみ出力し、説明・注釈は不要

英文: {}
日本語訳:"#,
            sentence
        );
        let translation = generate(prompt, 60).await?;
        // "日本語訳:" が含まれる場合は除去
        let clean = translation
            .trim_start_matches("日本語訳:")
            .trim_start_matches("「")
            .trim_end_matches("」")
            .trim()
            .to_string();
        results.push(clean);
    }
    Ok(results)
}

pub async fn summarize(sentences: &[String]) -> Result<String, String> {
    let text: String = sentences.join(" ").chars().take(8000).collect();
    let prompt = format!(
        r#"あなたはプロの編集者です。以下の英語テキストの内容を日本語で詳しく要約してください。

ルール：
- 400〜500文字程度で詳しくまとめる
- トピック・主張・具体例・結論を含める
- 箇条書きではなく、読みやすい文章で書く
- 日本語の要約のみ出力し、説明や前置きは不要

英語テキスト:
{}

日本語要約:"#,
        text
    );
    let result = generate(prompt, 180).await?;
    let clean = result
        .trim_start_matches("日本語要約:")
        .trim()
        .to_string();
    Ok(clean)
}
