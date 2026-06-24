use std::error::Error;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::config::AppConfig;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct AiTestResult {
    pub ok: bool,
    pub message: String,
}

pub struct AiSession {
    client: Client,
    giga_client: Client,
    messages: Vec<AiMessage>,
    system_prompt: String,
    cfg: AppConfig,
}

impl AiSession {
    pub fn new(cfg: AppConfig) -> Self {
        let system = cfg.resolved_system_prompt();
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .unwrap_or_else(|_| Client::new());
        // Отдельный клиент для GigaChat (российские сертификаты)
        let giga_client = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            giga_client,
            messages: Vec::new(),
            system_prompt: system,
            cfg,
        }
    }

    pub fn configure(&mut self, cfg: AppConfig) {
        self.system_prompt = cfg.resolved_system_prompt();
        self.cfg = cfg;
        // Пересоздаём клиенты
        self.client = Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .unwrap_or_else(|_| Client::new());
        self.giga_client = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .unwrap_or_else(|_| Client::new());
    }

    pub fn ask(&mut self, question: &str) -> Result<String> {
        let q = question.trim();
        if q.is_empty() {
            return Err(anyhow!("Пустой запрос"));
        }
        self.messages.push(AiMessage {
            role: "user".into(),
            content: q.into(),
        });
        self.trim_history();

        let answer = match self.cfg.provider.to_lowercase().as_str() {
            "gigachat" => self.ask_gigachat()?,
            "local llms" => self.ask_local_llm()?,
            _ => self.ask_openai_compatible()?,
        };

        self.messages.push(AiMessage {
            role: "assistant".into(),
            content: answer.clone(),
        });
        self.trim_history();
        Ok(answer)
    }

    pub fn test_connection(&self) -> AiTestResult {
        match self.cfg.provider.to_lowercase().as_str() {
            "gigachat" => self.test_gigachat(),
            "local llms" => self.test_local_llm(),
            _ => self.test_openai_compatible(),
        }
    }

    fn build_payload(&self) -> Vec<AiMessage> {
        let mut out = Vec::with_capacity(self.messages.len() + 1);
        if !self.system_prompt.is_empty() {
            out.push(AiMessage {
                role: "system".into(),
                content: self.system_prompt.clone(),
            });
        }
        // Добавляем сообщения
        out.extend(self.messages.iter().cloned());
        out
    }

    fn trim_history(&mut self) {
        const MAX: usize = 20;
        if self.messages.len() > MAX {
            let drop = self.messages.len() - MAX;
            self.messages.drain(0..drop);
        }
    }

    fn ask_openai_compatible(&self) -> Result<String> {
        if self.cfg.api_key.trim().is_empty() {
            return Err(anyhow!("Не указан API Key"));
        }
        // Санитизируем API key
        let clean_key: String = self.cfg.api_key.chars()
            .filter(|c| c.is_ascii_graphic() || *c == ' ')
            .collect::<String>()
            .trim()
            .to_string();
        if clean_key.is_empty() {
            return Err(anyhow!("API Key содержит только недопустимые символы"));
        }
        let url = format!("{}/chat/completions", self.cfg.base_url.trim_end_matches("/"));
        let resp = self
            .client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", clean_key))
            .header(CONTENT_TYPE, "application/json")
            .json(&json!({
                "model": self.cfg.model,
                "messages": self.build_payload(),
                "temperature": 0.3,
            }))
            .send()
            .map_err(|e| {
                let mut msg = format!("Ошибка соединения с {url}: {e}");
                let mut src = e.source();
                while let Some(s) = src {
                    msg.push_str(&format!("\n  → причина: {s}"));
                    src = s.source();
                }
                anyhow!(msg)
            })?
            .error_for_status()
            .map_err(|e| {
                let status = e.status().map(|s| s.to_string()).unwrap_or_else(|| "?".into());
                let msg = format!("HTTP ошибка {status} от {url}\n  → проверьте API Key, Base URL и модель");
                anyhow!(msg)
            })?;

        let body: OpenAiResponse = resp.json().map_err(|e| {
            anyhow!("Не удалось разобрать ответ AI (JSON): {e}")
        })?;
        body.choices
            .first()
            .map(|c| c.message.content.clone())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("Пустой ответ AI — модель вернула 0 choices"))
    }

    fn ask_local_llm(&self) -> Result<String> {
        let url = format!(
            "http://{}:{}/v1/chat/completions",
            self.cfg.local_llm_address,
            self.cfg.local_llm_port
        );
        let resp = self
            .client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .json(&json!({
                "model": self.cfg.model,
                "messages": self.build_payload(),
                "temperature": 0.3,
            }))
            .send()
            .map_err(|e| anyhow!("Local LLM недоступен: {e}"))?
            .error_for_status()?;

        let body: OpenAiResponse = resp.json()?;
        body.choices
            .first()
            .map(|c| c.message.content.clone())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("Пустой ответ от Local LLM"))
    }

    fn ask_gigachat(&self) -> Result<String> {
        let token = gigachat_token(&self.giga_client, &self.cfg)?;
        let resp = self
            .giga_client
            .post("https://gigachat.devices.sberbank.ru/api/v1/chat/completions")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(CONTENT_TYPE, "application/json")
            .json(&json!({
                "model": self.cfg.model,
                "messages": self.build_payload(),
                "temperature": 0.3,
            }))
            .send()?
            .error_for_status()?;

        let body: OpenAiResponse = resp.json()?;
        body.choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("Пустой ответ GigaChat"))
    }

    fn test_openai_compatible(&self) -> AiTestResult {
        match self.ask_openai_compatible() {
            Ok(_) => AiTestResult {
                ok: true,
                message: "Соединение успешно".into(),
            },
            Err(e) => AiTestResult {
                ok: false,
                message: e.to_string(),
            },
        }
    }

    fn test_local_llm(&self) -> AiTestResult {
        let url = format!(
            "http://{}:{}/v1/chat/completions",
            self.cfg.local_llm_address,
            self.cfg.local_llm_port
        );
        match self
            .client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .json(&json!({
                "model": self.cfg.model,
                "messages": &vec![AiMessage {
                    role: "user".into(),
                    content: "Hello".into(),
                }],
                "temperature": 0.3,
            }))
            .send()
        {
            Ok(resp) if resp.status().is_success() => AiTestResult {
                ok: true,
                message: "Local LLM: Соединение успешно".into(),
            },
            Ok(resp) => AiTestResult {
                ok: false,
                message: format!("Local LLM HTTP: {}", resp.status()),
            },
            Err(e) => AiTestResult {
                ok: false,
                message: format!("Local LLM недоступен: {e}"),
            },
        }
    }

    fn test_gigachat(&self) -> AiTestResult {
        match gigachat_token(&self.giga_client, &self.cfg) {
            Ok(_) => AiTestResult {
                ok: true,
                message: "GigaChat: соединение успешно".into(),
            },
            Err(e) => AiTestResult {
                ok: false,
                message: e.to_string(),
            },
        }
    }

}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    content: String,
}

fn gigachat_token(client: &Client, cfg: &AppConfig) -> Result<String> {
    if cfg.gigachat_auth_key.trim().is_empty() {
        return Err(anyhow!("GigaChat: не указан Authorization Key"));
    }

    // Authorization Key в Basic
    let resp = client
        .post("https://ngw.devices.sberbank.ru:9443/api/v2/oauth")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .header("RqUID", Uuid::new_v4().to_string())
        .header(AUTHORIZATION, format!("Basic {}", cfg.gigachat_auth_key.trim()))
        .body(format!("scope={}", cfg.gigachat_scope))
        .send()
        .map_err(|e| anyhow!("GigaChat OAuth: соединение не удалось — {e}"))?
        .error_for_status()
        .map_err(|e| {
            let status = e.status().map(|s| s.to_string()).unwrap_or_else(|| "?".into());
            anyhow!("GigaChat OAuth: HTTP {status} — проверьте Authorization Key и Scope")
        })?;

    #[derive(Deserialize)]
    struct TokenResp {
        access_token: String,
    }

    let body: TokenResp = resp.json().map_err(|e| {
        anyhow!("GigaChat OAuth: не удалось разобрать ответ — {e}")
    })?;

    if body.access_token.is_empty() {
        return Err(anyhow!("GigaChat OAuth: получен пустой токен"));
    }

    Ok(body.access_token)
}

pub fn spawn_ai_request(
    cfg: AppConfig,
    history: Arc<Mutex<AiSession>>,
    question: String,
    tx: crossbeam_channel::Sender<AiEvent>,
) {
    std::thread::Builder::new()
        .name("ai-worker".into())
        .spawn(move || {
            let result = {
                let mut session = history.lock().unwrap();
                session.configure(cfg);
                session.ask(&question)
            };
            match result {
                Ok(answer) => {
                    let _ = tx.send(AiEvent::Answer(answer));
                }
                Err(err) => {
                    let _ = tx.send(AiEvent::Error(err.to_string()));
                }
            }
        })
        .ok();
}

#[derive(Clone, Debug)]
pub enum AiEvent {
    Answer(String),
    Error(String),
}
