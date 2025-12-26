use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::RwLock;

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcMessage {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<serde_json::Value>,
    #[serde(flatten)]
    rest: serde_json::Map<String, serde_json::Value>,
}

struct Proxy {
    url: String,
    agent: ureq::Agent,
    session_id: RwLock<Option<String>>,
}

impl Proxy {
    fn new(url: String) -> Self {
        Self {
            url,
            agent: ureq::Agent::new_with_defaults(),
            session_id: RwLock::new(None),
        }
    }

    fn send(&self, msg: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let mut req = self
            .agent
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        if let Some(ref id) = *self.session_id.read().unwrap() {
            req = req.header("Mcp-Session-Id", id);
        }

        let res = req.send(msg)?;

        if let Some(id) = res.headers().get("Mcp-Session-Id")
            && let Ok(s) = id.to_str()
        {
            *self.session_id.write().unwrap() = Some(s.to_string());
        }

        let status = res.status();
        if status == 404 {
            *self.session_id.write().unwrap() = None;
            return Err("Session expired".into());
        }

        let content_type = res
            .headers()
            .get("Content-Type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if status == 202 {
            Ok(None)
        } else if content_type.contains("text/event-stream") {
            let reader = BufReader::new(res.into_body().into_reader());
            let msgs: Vec<_> = reader
                .lines()
                .map_while(Result::ok)
                .filter_map(|l| l.strip_prefix("data: ").filter(|d| !d.is_empty()).map(String::from))
                .collect();
            Ok((!msgs.is_empty()).then(|| msgs.join("\n")))
        } else {
            let body = res.into_body().read_to_string()?;
            Ok((!body.is_empty()).then_some(body))
        }
    }
}

fn write_stdout(msg: &str) {
    let mut out = io::stdout().lock();
    for line in msg.lines().filter(|l| !l.trim().is_empty()) {
        let _ = writeln!(out, "{}", line);
    }
    let _ = out.flush();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let Some(url) = args.get(1) else {
        eprintln!("Usage: {} <server-url>", args[0]);
        std::process::exit(1);
    };

    let proxy = Proxy::new(url.clone());

    for line in io::stdin().lock().lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match proxy.send(trimmed) {
            Ok(Some(res)) => write_stdout(&res),
            Ok(None) => {}
            Err(e) => {
                if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(trimmed)
                    && let Some(id) = msg.id
                {
                    let err = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32603, "message": e.to_string() }
                    });
                    write_stdout(&err.to_string());
                }
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}
