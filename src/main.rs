use serde_json::{json, Value};
use std::{
    collections::HashMap,
    env, fs,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

// users.json: участники групп; memory.json: персональная история общения.
type Users = HashMap<String, HashMap<String, String>>;
type Memories = HashMap<String, Vec<String>>;
const USERS_DB: &str = "users.json";
const MEMORY_DB: &str = "memory.json";

const SYSTEM: &str = "Ты — MOGGER AI, активный и дерзкий, но адекватный участник Telegram-группы про looksmaxxing и mogging. \
Отвечай по-русски, живо, остроумно, уверенно, обычно 2–6 предложений. Используй сленг уместно: mog, aura, frame, looksmaxxing, grooming, W/L, но не вставляй его в каждую фразу. \
Твоя задача — создавать содержательную интеракцию: спорить с тезисами, задавать конкретные вопросы, предлагать батлы, замечать противоречия и продолжать прошлые темы. \
Не выдумывай факты о внешности человека, если не видел фото; не ставь диагнозы и не выдавай PSL/рейтинги за научный факт. \
Критикуй идеи, аргументы, стиль рассуждения и добровольно предоставленные образы, но не унижай тело, лицо, инвалидность, происхождение или другие личные особенности. \
Советы только безопасные: сон, уход за кожей, причёска, одежда, осанка, тренировки без экстремизма и уверенность. Никаких стероидов, опасных процедур, экстремальных диет или костоломания. \
Если пользователь прав — признай это. Не соглашайся автоматически и не спорь ради спора. Не раскрывай и не перечисляй сохранённую персональную память; используй её естественно и только по теме.";

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn load_users() -> Users {
    fs::read_to_string(USERS_DB)
        .ok()
        .and_then(|s| serde_json::from_str::<Users>(&s).ok())
        .unwrap_or_default()
}

fn load_memories() -> Memories {
    fs::read_to_string(MEMORY_DB)
        .ok()
        .and_then(|s| serde_json::from_str::<Memories>(&s).ok())
        .unwrap_or_default()
}

fn save_users(data: &Users) {
    if let Ok(s) = serde_json::to_string_pretty(data) {
        if let Err(e) = fs::write(USERS_DB, s) { eprintln!("Не удалось сохранить {USERS_DB}: {e}"); }
    }
}

fn save_memories(data: &Memories) {
    if let Ok(s) = serde_json::to_string_pretty(data) {
        if let Err(e) = fs::write(MEMORY_DB, s) { eprintln!("Не удалось сохранить {MEMORY_DB}: {e}"); }
    }
}

async fn tg(c: &reqwest::Client, token: &str, method: &str, body: Value) -> Value {
    match c.post(format!("https://api.telegram.org/bot{token}/{method}"))
        .json(&body)
        .timeout(Duration::from_secs(70))
        .send().await
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
        let r = c.post("https://text.pollinations.ai/openai")
            .timeout(Duration::from_secs(90))
            .json(&json!({"model":"openai","messages":[
                {"role":"system","content":system},
                {"role":"user","content":prompt}
            ]}))
            .send().await;
        match r {
            Ok(resp) => {
                let v: Value = resp.json().await.unwrap_or(json!({}));
                if let Some(t) = v["choices"][0]["message"]["content"].as_str() {
                    if !t.trim().is_empty() { return t.chars().take(3900).collect(); }
                }
                eprintln!("ai fail: {v}");
            }
            Err(e) => eprintln!("ai fail: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    "ИИ сейчас недоступен. Продолжим батл чуть позже.".into()
}

async fn send(c: &reqwest::Client, token: &str, chat_id: i64, text: &str, reply: Option<i64>) {
    let mut body = json!({"chat_id":chat_id,"text":esc(text),"parse_mode":"HTML"});
    if let Some(id) = reply { body["reply_to_message_id"] = json!(id); }
    let _ = tg(c, token, "sendMessage", body).await;
}

fn remember(memories: &mut Memories, chat_id: i64, user_id: i64, entry: String) {
    let key = format!("{chat_id}:{user_id}");
    let history = memories.entry(key).or_default();
    history.push(entry);
    // Ограничиваем размер персональной памяти, чтобы файл не рос бесконечно.
    if history.len() > 16 { history.drain(0..history.len() - 16); }
}

fn should_engage(
    text: &str,
    addressed_to_bot: bool,
    chat_id: i64,
    user_id: i64,
    last_chat_reply: &HashMap<i64, u64>,
    last_user_reply: &HashMap<String, u64>,
) -> bool {
    let lower = text.to_lowercase();
    let direct = addressed_to_bot || text.contains('?') || [
        "могг", "моггинг", "лукс", "looksmax", "psl", "внешност", "причес",
        "причёск", "стил", "аур", "фрейм", "груминг", "кож", "спорт", "зал",
        "кто прав", "не соглас", "батл", "battle", "face", "hairmax", "gymmax",
    ].iter().any(|w| lower.contains(w));

    let t = now();
    // Maximum: бот реагирует на прямое обращение/тему, а в остальном сам вступает примерно в каждый третий диалог.
    // Ограничители не дают ему заспамить группу: не чаще раза в 10 секунд на чат и 35 секунд на автора.
    let chat_ready = t.saturating_sub(*last_chat_reply.get(&chat_id).unwrap_or(&0)) >= 10;
    let user_key = format!("{chat_id}:{user_id}");
    let user_ready = t.saturating_sub(*last_user_reply.get(&user_key).unwrap_or(&0)) >= 35;
    if !chat_ready || !user_ready { return false; }
    if direct { return true; }

    let seed = text.bytes().fold(t, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    seed % 3 == 0
}

fn pick_two(users: &Users, chat_id: i64) -> Option<((String, String), (String, String))> {
    let mut people: Vec<(String, String)> = users.get(&chat_id.to_string())?
        .iter().map(|(id, name)| (id.clone(), name.clone())).collect();
    people.sort_by(|a, b| a.0.cmp(&b.0));
    if people.len() < 2 { return None; }
    let seed = now() as usize;
    let first = seed % people.len();
    let mut second = (seed / 7 + 1) % people.len();
    if second == first { second = (first + 1) % people.len(); }
    Some((people[first].clone(), people[second].clone()))
}

fn mention(id: &str, name: &str) -> String {
    let safe_id = id.parse::<i64>().unwrap_or(0);
    format!("<a href=\"tg://user?id={safe_id}\">{}</a>", esc(name))
}

#[tokio::main]
async fn main() {
    let token = env::var("TG_TOKEN").expect("TG_TOKEN не задан");
    let c = reqwest::Client::builder().user_agent("MoggerAI/1.0").build().unwrap();
    let mut users: Users = load_users();
    let mut memories: Memories = load_memories();
    let mut last_chat_reply: HashMap<i64, u64> = HashMap::new();
    let mut last_user_reply: HashMap<String, u64> = HashMap::new();
    let mut offset: i64 = 0;

    let me = tg(&c, &token, "getMe", json!({})).await;
    let bot_id = me["result"]["id"].as_i64().unwrap_or(0);
    let bot_username = me["result"]["username"].as_str().unwrap_or("").to_lowercase();
    println!("MOGGER AI запущен. Бот: @{bot_username}");

    loop {
        let res = tg(&c, &token, "getUpdates", json!({
            "offset":offset,"timeout":50,"allowed_updates":["message"]
        })).await;
        let Some(updates) = res["result"].as_array() else {
            tokio::time::sleep(Duration::from_secs(3)).await;
            continue;
        };

        for update in updates {
            offset = update["update_id"].as_i64().unwrap_or(offset) + 1;
            let m = &update["message"];
            let Some(chat_id) = m["chat"]["id"].as_i64() else { continue };
            let Some(user_id) = m["from"]["id"].as_i64() else { continue };
            if m["from"]["is_bot"].as_bool() == Some(true) { continue; }
            let message_id = m["message_id"].as_i64();
            let chat_type = m["chat"]["type"].as_str().unwrap_or("");
            let is_group = chat_type == "group" || chat_type == "supergroup";
            let name = m["from"]["first_name"].as_str().unwrap_or("боец");
            let Some(text) = m["text"].as_str() else { continue };

            if is_group {
                users.entry(chat_id.to_string()).or_default()
                    .insert(user_id.to_string(), name.to_string());
                save_users(&users);
            }

            let reply_to_bot = m["reply_to_message"]["from"]["id"].as_i64() == Some(bot_id);
            let mentioned = !bot_username.is_empty() && text.to_lowercase().contains(&format!("@{bot_username}"));
            let addressed = reply_to_bot || mentioned;

            if text.starts_with('/') {
                let (command, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
                let command = command.split('@').next().unwrap_or("").to_lowercase();
                let rest = rest.trim();
                match command.as_str() {
                    "/start" | "/help" => {
                        send(&c, &token, chat_id,
                            "🔥 MOGGER AI — активный участник чата\n\n/battle — вызвать двух случайных участников на батл\n/mog [вопрос] — моггинг и looksmaxxing\n/ask [вопрос] — задать вопрос\n/profile — показать, что бот помнит о вашем общении\n/forgetme — удалить вашу персональную историю\n\nЯ могу сам вступать в обсуждения, спорить по существу и предлагать батлы. В группе отключите Privacy Mode через @BotFather, чтобы бот видел обычные сообщения.", message_id).await;
                    }
                    "/battle" => {
                        if let Some(((id1, n1), (id2, n2))) = pick_two(&users, chat_id) {
                            let topic = if rest.is_empty() { "Style / Aura / Grooming" } else { rest };
                            let battle = format!(
                                "⚔️ MOGGING BATTLE!\n\n{} VS {}\n\n{} — тема сегодняшнего батла.\n\nРаунды:\n1️⃣ Стиль — цельность образа и уместность.\n2️⃣ Grooming — аккуратность и уход.\n3️⃣ Aura — уверенная подача и аргументация.\n\nУчастники, принимаете вызов? Остальные — голосуйте и аргументируйте W/L. Без унижения внешности: разносим аргументы, а не людей. 🏆",
                                mention(&id1, &n1), mention(&id2, &n2), esc(topic)
                            );
                            let mut body = json!({"chat_id":chat_id,"text":battle,"parse_mode":"HTML"});
                            if let Some(mid) = message_id { body["reply_to_message_id"] = json!(mid); }
                            let _ = tg(&c, &token, "sendMessage", body).await;
                        } else {
                            send(&c, &token, chat_id, "Для батла нужны хотя бы два участника, которые уже писали в группе.", message_id).await;
                        }
                    }
                    "/mog" | "/ask" => {
                        let question = if rest.is_empty() { "Дай мне полезный looksmaxxing-челлендж на сегодня и задай уточняющий вопрос." } else { rest };
                        let key = format!("{chat_id}:{user_id}");
                        let history = memories.get(&key).cloned().unwrap_or_default().join("\n");
                        let prompt = format!("Контекст прошлых разговоров с {name}:\n{history}\n\nНовое сообщение: {question}\nОтветь лично этому участнику, продолжи контекст и закончи конкретным вопросом или вызовом, если это уместно.");
                        let answer = ai(&c, SYSTEM, &prompt).await;
                        send(&c, &token, chat_id, &answer, message_id).await;
                        remember(&mut memories, chat_id, user_id, format!("{name}: {question}\nMOGGER AI: {answer}"));
                        save_memories(&memories);
                    }
                    "/profile" => {
                        let key = format!("{chat_id}:{user_id}");
                        let history = memories.get(&key).cloned().unwrap_or_default();
                        let response = if history.is_empty() {
                            "Пока у меня нет истории нашего общения. Напиши что-нибудь — начнём собирать контекст.".to_string()
                        } else {
                            format!("🧠 Я помню последние темы нашего общения:\n{}", history.iter().rev().take(5).rev().cloned().collect::<Vec<_>>().join("\n\n"))
                        };
                        send(&c, &token, chat_id, &response, message_id).await;
                    }
                    "/forgetme" => {
                        memories.remove(&format!("{chat_id}:{user_id}"));
                        save_memories(&memories);
                        send(&c, &token, chat_id, "Твоя персональная история общения в этом чате удалена.", message_id).await;
                    }
                    _ => {}
                }
                continue;
            }

            // Сохраняем реплику каждого участника для персонального контекста.
            let user_key = format!("{chat_id}:{user_id}");
            remember(&mut memories, chat_id, user_id, format!("{name}: {}", text.chars().take(500).collect::<String>()));
            save_memories(&memories);

            if is_group && should_engage(text, addressed, chat_id, user_id, &last_chat_reply, &last_user_reply) {
                let history = memories.get(&user_key).cloned().unwrap_or_default();
                let context = history.iter().rev().take(10).cloned().collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
                let prompt = format!(
                    "Ты сейчас в Telegram-группе. Участник: {name} (id {user_id}).\nПерсональный контекст последних сообщений этого участника и группы:\n{context}\n\nПоследнее сообщение участника: {text}\n\nОтветь как живой участник чата: по существу отреагируй на сообщение, при необходимости возрази с объяснением, задай один цепляющий уточняющий вопрос или предложи дружеский батл. Не начинай каждый ответ одинаково. Не выдумывай факты. Максимум 6 коротких предложений."
                );
                let answer = ai(&c, SYSTEM, &prompt).await;
                send(&c, &token, chat_id, &answer, message_id).await;
                remember(&mut memories, chat_id, user_id, format!("MOGGER AI: {answer}"));
                save_memories(&memories);
                let t = now();
                last_chat_reply.insert(chat_id, t);
                last_user_reply.insert(user_key, t);
            }
        }
    }
}
