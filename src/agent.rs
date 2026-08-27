//! Shared Unpeel App-to-agent handoff primitives.
//!
//! The Host's unified MCP remains the authority for peer discovery and
//! same-group write policy. Apps only ask it for the best nearby agent and
//! paste a reference into that agent's input without submitting it.

use std::collections::HashSet;
use std::fmt;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use serde_json::{Value, json};

/// Error returned when no eligible agent exists or the Host MCP is
/// unavailable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentError(String);

impl AgentError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for AgentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AgentError {}

/// Whether this process is running inside a hosted Unpeel session.
#[must_use]
pub fn is_hosted() -> bool {
    std::env::var("UNPEEL_SESSION_ID").is_ok_and(|session_id| !session_id.trim().is_empty())
}

/// A control-safe, absolute path token suitable for an agent input.
///
/// JSON string escaping keeps newlines and other unusual filename bytes from
/// becoming terminal control input. Non-UTF-8 bytes are represented lossily;
/// Unpeel's supported macOS filesystems normally provide UTF-8 names.
#[must_use]
pub fn path_reference(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let quoted = serde_json::to_string(absolute.to_string_lossy().as_ref())
        .unwrap_or_else(|_| "\"\"".to_owned());
    format!("[path: {quoted}]")
}

/// OSC 52 clipboard-copy sequence for an App's standalone fallback.
#[must_use]
pub fn clipboard_sequence(text: &str) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    format!("\x1b]52;c;{encoded}\x07")
}

/// Paste text into the best agent in this App's sidebar group.
///
/// A direct pane neighbor wins, then a recognized agent runtime, then an
/// older command-derived provider match. Settled agents are preferred. The
/// text is deliberately not submitted, leaving the user in control of the
/// final prompt.
pub fn send_to_agent(text: &str) -> Result<String, AgentError> {
    if !is_hosted() {
        return Err(AgentError::new("not inside an Unpeel session"));
    }
    let mut client = McpClient::spawn()?;
    let (target_id, label) = resolve_agent(&mut client)?;
    client.call_tool(
        "sessions",
        &json!({
            "action": "send_text",
            "session_id": target_id,
            "text": format!("{text} "),
            "submit": false,
        }),
    )?;
    Ok(label)
}

/// Cached, non-blocking peer discovery for context menus plus synchronous
/// send helpers for the selected action.
///
/// `refresh` never blocks the UI thread and coalesces concurrent probes.
/// `send_text` re-resolves at click time so a stale menu label cannot send to
/// a departed session.
#[derive(Clone, Default)]
pub struct AgentBridge {
    label: Arc<Mutex<Option<String>>>,
    probing: Arc<AtomicBool>,
}

impl AgentBridge {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Last known peer label. `None` means the menu should offer a copy
    /// fallback rather than promise that an agent is available.
    #[must_use]
    pub fn label(&self) -> Option<String> {
        self.label.lock().ok()?.clone()
    }

    /// Refresh the cached label off the UI thread, if hosted by Unpeel.
    pub fn refresh(&self) {
        if !is_hosted() || self.probing.swap(true, Ordering::SeqCst) {
            return;
        }
        let label = Arc::clone(&self.label);
        let probing = Arc::clone(&self.probing);
        std::thread::spawn(move || {
            let found = McpClient::spawn()
                .and_then(|mut client| resolve_agent(&mut client))
                .ok()
                .map(|(_, label)| label);
            if let Ok(mut slot) = label.lock() {
                *slot = found;
            }
            probing.store(false, Ordering::SeqCst);
        });
    }

    /// Paste text into the current target and update the cached label.
    pub fn send_text(&self, text: &str) -> Result<String, AgentError> {
        let label = send_to_agent(text)?;
        if let Ok(mut slot) = self.label.lock() {
            *slot = Some(label.clone());
        }
        Ok(label)
    }

    /// Paste a safe absolute path reference into the current target.
    pub fn send_path(&self, path: impl AsRef<Path>) -> Result<String, AgentError> {
        self.send_text(&path_reference(path))
    }
}

fn resolve_agent(client: &mut McpClient) -> Result<(String, String), AgentError> {
    let group = client.call_tool(
        "sessions",
        &json!({ "action": "list_group", "include_exited": false }),
    )?;
    let sessions = group
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let group_ids = sessions
        .iter()
        .map(|session| string_field(session, "id"))
        .collect::<HashSet<_>>();

    let neighbor = client
        .call_tool("sessions", &json!({ "action": "current" }))
        .ok()
        .and_then(|current| adjacent_target(&current, &group_ids));
    if let Some(target) = neighbor {
        return Ok(target);
    }

    let recognized = client
        .call_tool("agents", &json!({ "action": "list" }))
        .ok()
        .and_then(|agents| recognized_target(&agents, &group_ids));
    if let Some(target) = recognized {
        return Ok(target);
    }

    provider_target(&sessions).ok_or_else(|| AgentError::new("no agent session in this group"))
}

fn adjacent_target(current: &Value, group_ids: &HashSet<String>) -> Option<(String, String)> {
    let neighbors = current.pointer("/pane_context/neighbors")?;
    let mut candidates = ["left", "right", "up", "down"]
        .iter()
        .filter_map(|direction| neighbors.get(*direction))
        .filter(|entry| {
            string_field(entry, "kind") == "agent"
                && group_ids.contains(&string_field(entry, "session_id"))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|entry| settled_rank(&string_field(entry, "activity_status")));
    candidates.first().map(|entry| {
        let runtime = string_field(entry, "runtime_id");
        (
            string_field(entry, "session_id"),
            label_or(entry, "label", &runtime),
        )
    })
}

fn recognized_target(agents: &Value, group_ids: &HashSet<String>) -> Option<(String, String)> {
    let mut candidates = agents
        .get("agents")
        .and_then(Value::as_array)?
        .iter()
        .filter(|agent| {
            agent.get("self").and_then(Value::as_bool) != Some(true)
                && agent.get("agent_ref").is_some_and(|reference| {
                    group_ids.contains(&string_field(reference, "session_id"))
                })
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|agent| settled_rank(&string_field(agent, "activity_status")));
    candidates.first().and_then(|agent| {
        let reference = agent.get("agent_ref")?;
        let runtime = string_field(agent, "runtime_id");
        Some((
            string_field(reference, "session_id"),
            label_or(agent, "label", &runtime),
        ))
    })
}

fn provider_target(sessions: &[Value]) -> Option<(String, String)> {
    let mut candidates = sessions
        .iter()
        .filter(|session| {
            let provider = string_field(session, "provider");
            string_field(session, "state") == "running"
                && !provider.is_empty()
                && provider != "shell"
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|session| settled_rank(&string_field(session, "activity_status")));
    candidates.first().map(|session| {
        let provider = string_field(session, "provider");
        (
            string_field(session, "id"),
            label_or(session, "label", &provider),
        )
    })
}

fn settled_rank(status: &str) -> u8 {
    match status {
        "idle" | "done" => 0,
        "blocked" => 1,
        _ => 2,
    }
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn label_or(value: &Value, key: &str, fallback: &str) -> String {
    let label = string_field(value, key);
    if label.is_empty() {
        fallback.to_owned()
    } else {
        label
    }
}

struct McpClient {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
    next_id: u64,
}

impl McpClient {
    fn spawn() -> Result<Self, AgentError> {
        let mut child = Command::new("unpeel-host")
            .arg("__mcp__")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| AgentError::new(format!("unpeel-host not available: {error}")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AgentError::new("mcp stdout unavailable"))?;
        let mut client = Self {
            child,
            reader: BufReader::new(stdout),
            next_id: 1,
        };
        client.request(
            "initialize",
            json!({ "protocolVersion": "2025-06-18", "capabilities": {} }),
        )?;
        Ok(client)
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, AgentError> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| AgentError::new("mcp stdin closed"))?;
        writeln!(stdin, "{body}")
            .map_err(|error| AgentError::new(format!("mcp write failed: {error}")))?;
        stdin
            .flush()
            .map_err(|error| AgentError::new(format!("mcp flush failed: {error}")))?;

        let mut line = String::new();
        loop {
            line.clear();
            let read = self
                .reader
                .read_line(&mut line)
                .map_err(|error| AgentError::new(format!("mcp read failed: {error}")))?;
            if read == 0 {
                return Err(AgentError::new("mcp server exited"));
            }
            let Ok(message) = serde_json::from_str::<Value>(line.trim()) else {
                continue;
            };
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(AgentError::new(format!("mcp error: {error}")));
            }
            return Ok(message.get("result").cloned().unwrap_or_default());
        }
    }

    fn call_tool(&mut self, tool: &str, arguments: &Value) -> Result<Value, AgentError> {
        let result = self.request(
            "tools/call",
            json!({ "name": tool, "arguments": arguments }),
        )?;
        let text = result
            .pointer("/content/0/text")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if result.get("isError").and_then(Value::as_bool) == Some(true) {
            return Err(AgentError::new(text));
        }
        Ok(serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_owned())))
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        self.child.stdin.take();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_tokens_are_absolute_and_control_safe() {
        let token = path_reference(Path::new("folder/odd\nname"));
        assert!(token.starts_with("[path: \"/"));
        assert!(token.contains("odd\\nname"));
        assert!(!token.contains("odd\nname"));
    }

    #[test]
    fn clipboard_sequence_uses_osc_52() {
        assert_eq!(clipboard_sequence("hello"), "\x1b]52;c;aGVsbG8=\x07");
    }

    #[test]
    fn adjacent_idle_agent_wins_inside_the_group() {
        let current = json!({
            "pane_context": { "neighbors": {
                "left": { "kind": "agent", "session_id": "busy", "label": "Busy", "activity_status": "busy" },
                "right": { "kind": "agent", "session_id": "idle", "runtime_id": "claude", "activity_status": "idle" },
                "down": { "kind": "agent", "session_id": "other-group", "label": "Wrong", "activity_status": "idle" }
            }}
        });
        let group = HashSet::from(["busy".to_owned(), "idle".to_owned()]);
        assert_eq!(
            adjacent_target(&current, &group),
            Some(("idle".to_owned(), "claude".to_owned()))
        );
    }

    #[test]
    fn provider_fallback_excludes_shells_and_prefers_settled() {
        let sessions = vec![
            json!({ "id": "shell", "provider": "shell", "state": "running", "activity_status": "idle" }),
            json!({ "id": "busy", "provider": "codex", "state": "running", "activity_status": "busy" }),
            json!({ "id": "idle", "provider": "claude", "label": "Claude", "state": "running", "activity_status": "done" }),
        ];
        assert_eq!(
            provider_target(&sessions),
            Some(("idle".to_owned(), "Claude".to_owned()))
        );
    }
}
