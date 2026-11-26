use serde_json::{json, Value};
use std::{collections::HashMap, env, fs, time::Duration};

type Users = HashMap<String, HashMap<String, String>>;
const DB: &str = "users.json";
const SYSTEM: &str = "Ты - дерзкий, но добрый бот-тренер по моггингу и луксмаксингу в Telegram-чате. \
Отвечай по-русски, коротко (до 8 предложений), с юмором и сленгом (мог, аура, сигма, фрейм). \
Давай только безопасные советы: уход за кожей, сон, вода, осанка, стиль, причёска, спорт, уверенность. \
Никогда не советуй экстремальные диеты, стероиды, опасные процедуры или костоломание. \
Не оскорбляй внешность реальных людей, батлы только шуточные и дружеские.";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

async fn tg(c: &reqwest::Client, token: &str, method: &str, body: Value) -> Value {
    match c
        .post(format!("https://api.telegram.org/bot{token}/{method}"))
        .json(&body)
        .timeout(Duration::from_secs(70))
        .send()
        .await
    {
        Ok(r) => r.json().await.unwrap_or(json!({})),
        Err(e) => {
            eprintln!("tg error {method}: {e}");
            json!({})
        }
    }
}

async fn ai(c: &reqwest::Client, system: &str, prompt: &str) -> String {
    for _ in 0..2 {
        let r = c
            .post("https://text.pollinations.ai/openai")
            .timeout(Duration::from_secs(90))
            .json(&json!({"model": "openai", "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": prompt}]}))
            .send()
            .await;
        match r {
            Ok(resp) => {
                let v: Value = resp.json().await.unwrap_or(json!({}));
                if let Some(t) = v["choices"][0]["message"]["content"].as_str() {
                    if !t.trim().is_empty() {
                        return t.chars().take(3900).collect();
                    }
                }
                eprintln!("ai fail: {v}");
            }
            Err(e) => eprintln!("ai fail: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    "ИИ сейчас недоступен, попробуй чуть позже.".into()
}

async fn tags(c: &reqwest::Client, token: &str, users: &Users, chat_id: i64) -> String {
    let mut members: HashMap<String, String> =
        users.get(&chat_id.to_string()).cloned().unwrap_or_default();
    let adm = tg(c, token, "getChatAdministrators", json!({"chat_id": chat_id})).await;
    if let Some(list) = adm["result"].as_array() {
        for a in list {
            if a["user"]["is_bot"].as_bool() != Some(true) {
                if let (Some(id), Some(n)) =
                    (a["user"]["id"].as_i64(), a["user"]["first_name"].as_str())
                {
                    members.insert(id.to_string(), n.to_string());
                }
            }
        }
    }
    members
        .iter()
        .map(|(id, n)| format!("<a href=\"tg://user?id={id}\">{}</a>", esc(n)))
        .collect::<Vec<_>>()
        .join(" ")
}

async fn send(c: &reqwest::Client, token: &str, chat_id: i64, text: &str, reply: Option<i64>, tag_html: &str) {
    let mut body = json!({
        "chat_id": chat_id,
        "parse_mode": "HTML",
        "text": if tag_html.is_empty() { esc(text) } else { format!("{}\n\n{}", esc(text), tag_html) }
    });
    if let Some(r) = reply {
        body["reply_to_message_id"] = json!(r);
    }
    tg(c, token, "sendMessage", body).await;
}

#[tokio::main]
async fn main() {
    let token = env::var("TG_TOKEN").expect("TG_TOKEN не задан");
    let c = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()
        .unwrap();
    let mut users: Users = fs::read_to_string(DB)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut offset: i64 = 0;
    println!("Бот запущен");
