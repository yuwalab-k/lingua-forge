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
    std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "mitmul/plamo-2-translate".to_string())
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

pub async fn translate_single(sentence: &str) -> Result<String, String> {
    // plamo-2-translate は翻訳専用モデルのためシンプルなプロンプトで動作する
    let prompt = format!("{}", sentence);
    generate(prompt, 300).await
}
