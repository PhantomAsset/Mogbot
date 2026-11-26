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

loop {
        let res = tg(&c, &token, "getUpdates",
            json!({"offset": offset, "timeout": 50, "allowed_updates": ["message"]})).await;
        let Some(updates) = res["result"].as_array() else {
            tokio::time::sleep(Duration::from_secs(3)).await;
            continue;
        };

        for u in updates {
            offset = u["update_id"].as_i64().unwrap_or(offset) + 1;
            let m = &u["message"];
            let Some(cid) = m["chat"]["id"].as_i64() else { continue };
            let mid = m["message_id"].as_i64();
            let ctype = m["chat"]["type"].as_str().unwrap_or("");

            if (ctype == "group" || ctype == "supergroup")
                && m["from"]["is_bot"].as_bool() != Some(true)
            {
                if let (Some(id), Some(name)) =
                    (m["from"]["id"].as_i64(), m["from"]["first_name"].as_str())
                {
                    users.entry(cid.to_string()).or_default()
                        .insert(id.to_string(), name.to_string());
                    let _ = fs::write(DB, serde_json::to_string(&users).unwrap());
                }
            }

            let Some(text) = m["text"].as_str() else { continue };
            if !text.starts_with('/') { continue; }
            let (cmd, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
            let cmd = cmd.split('@').next().unwrap_or("").to_lowercase();
            let rest = rest.trim();

            match cmd.as_str() {
                "/battle" => {
                    let topic = if rest.is_empty() { "кто сильнее моггает" } else { rest };
                    let hype = ai(&c, SYSTEM, &format!(
                        "Напиши короткий боевой призыв на дружеский моггинг-батл всех участников чата. \
                         Тема: {topic}. Предложи 3 шуточных раунда (поза, стиль, аура). 5-6 строк.")).await;
                    let t = tags(&c, &token, &users, cid).await;
                    send(&c, &token, cid, &hype, None, &t).await;
                }
                "/mog" => {
                    let q = if rest.is_empty() { "Дай 5 базовых советов по моггингу для новичка" } else { rest };
                    let a = ai(&c, SYSTEM, q).await;
                    send(&c, &token, cid, &a, mid, "").await;
                }
                "/ask" => {
                    if rest.is_empty() {
                        send(&c, &token, cid, "Напиши вопрос: /ask что такое солянка?", mid, "").await;
                    } else {
                        let a = ai(&c, "Отвечай по-русски, кратко и по делу.", rest).await;
                        send(&c, &token, cid, &a, mid, "").await;
                    }
                }
                "/all" => {
                    let t = tags(&c, &token, &users, cid).await;
                    send(&c, &token, cid, if rest.is_empty() { "Все сюда!" } else { rest }, None, &t).await;
                }
                "/start" | "/help" => {
                    send(&c, &token, cid,
                        "/battle [тема] - вызвать всех на батл\n/mog вопрос - советы по моггингу\n/ask вопрос - обычный ИИ\n/all текст - позвать всех",
                        mid, "").await;
                }
                _ => {}
            }
        }
    }
}
