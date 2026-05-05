use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::async_runtime::JoinHandle;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

use crate::graph::GraphStore;
use crate::metadata::MetadataStore;
use crate::search::{hybrid_search, search_text};

const AGENT_API_ADDR: &str = "127.0.0.1:3737";

pub struct AgentApiState {
    server: Mutex<Option<AgentApiServer>>,
}

struct AgentApiServer {
    shutdown: oneshot::Sender<()>,
    handle: JoinHandle<()>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentApiStatus {
    pub running: bool,
    pub bind_address: String,
    pub local_only: bool,
}

#[derive(Debug, Deserialize)]
struct QueryRequest {
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct PathRequest {
    path: String,
}

#[derive(Debug, Deserialize)]
struct TargetRequest {
    target: String,
    limit: Option<usize>,
}

impl AgentApiState {
    pub fn new() -> Self {
        Self {
            server: Mutex::new(None),
        }
    }

    fn status(&self) -> Result<AgentApiStatus, String> {
        let server = self.server.lock().map_err(|error| error.to_string())?;
        Ok(AgentApiStatus {
            running: server.is_some(),
            bind_address: AGENT_API_ADDR.to_string(),
            local_only: true,
        })
    }
}

#[tauri::command]
pub async fn start_agent_api(
    app: tauri::AppHandle,
    state: tauri::State<'_, AgentApiState>,
    metadata_store: tauri::State<'_, MetadataStore>,
) -> Result<AgentApiStatus, String> {
    {
        let server = state.server.lock().map_err(|error| error.to_string())?;
        if server.is_some() {
            return state.status();
        }
    }

    let listener = TcpListener::bind(AGENT_API_ADDR)
        .await
        .map_err(|error| error.to_string())?;
    let metadata_store = metadata_store.inner().clone();
    let graph_store = GraphStore::open_default(&app).map_err(|error| error.to_string())?;
    let (shutdown, mut shutdown_rx) = oneshot::channel();

    let handle = tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    break;
                }
                accepted = listener.accept() => {
                    match accepted {
                        Ok((stream, _)) => {
                            let metadata_store = metadata_store.clone();
                            let graph_store = GraphStoreHandle::new(&graph_store);
                            handle_connection(stream, metadata_store, graph_store).await;
                        }
                        Err(error) => {
                            eprintln!("Agent API accept error: {error}");
                        }
                    }
                }
            }
        }
    });

    let mut server = state.server.lock().map_err(|error| error.to_string())?;
    *server = Some(AgentApiServer { shutdown, handle });
    state.status()
}

#[tauri::command]
pub async fn stop_agent_api(
    state: tauri::State<'_, AgentApiState>,
) -> Result<AgentApiStatus, String> {
    let server = {
        let mut server = state.server.lock().map_err(|error| error.to_string())?;
        server.take()
    };

    if let Some(server) = server {
        let _ = server.shutdown.send(());
        let _ = server.handle.await;
    }

    state.status()
}

#[tauri::command]
pub fn get_agent_api_status(
    state: tauri::State<'_, AgentApiState>,
) -> Result<AgentApiStatus, String> {
    state.status()
}

struct GraphStoreHandle<'a> {
    store: &'a GraphStore,
}

impl<'a> GraphStoreHandle<'a> {
    fn new(store: &'a GraphStore) -> Self {
        Self { store }
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    metadata_store: MetadataStore,
    graph_store: GraphStoreHandle<'_>,
) {
    let response = match read_request(&mut stream).await {
        Ok(request) => route_request(request, &metadata_store, graph_store.store).await,
        Err(error) => json_response(400, serde_json::json!({ "error": error })),
    };

    if let Err(error) = stream.write_all(response.as_bytes()).await {
        eprintln!("Agent API write error: {error}");
    }
}

struct HttpRequest {
    method: String,
    path: String,
    body: String,
}

async fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = vec![0_u8; 16 * 1024];
    let bytes_read = stream
        .read(&mut buffer)
        .await
        .map_err(|error| error.to_string())?;
    let raw = String::from_utf8_lossy(&buffer[..bytes_read]);
    let (headers, body) = raw
        .split_once("\r\n\r\n")
        .ok_or_else(|| "invalid HTTP request".to_string())?;
    let mut lines = headers.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "missing request line".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| "missing method".to_string())?
        .to_string();
    let path = request_parts
        .next()
        .ok_or_else(|| "missing path".to_string())?
        .to_string();

    Ok(HttpRequest {
        method,
        path,
        body: body.to_string(),
    })
}

async fn route_request(
    request: HttpRequest,
    metadata_store: &MetadataStore,
    graph_store: &GraphStore,
) -> String {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/status") => json_response(
            200,
            serde_json::json!({
                "status": "ready",
                "bindAddress": AGENT_API_ADDR,
                "localOnly": true
            }),
        ),
        ("POST", "/search") => match parse_json::<QueryRequest>(&request.body) {
            Ok(payload) => {
                match hybrid_search(metadata_store, &payload.query, payload.limit.unwrap_or(10))
                    .await
                {
                    Ok(results) => json_response(200, serde_json::json!({ "results": results })),
                    Err(error) => {
                        json_response(500, serde_json::json!({ "error": error.to_string() }))
                    }
                }
            }
            Err(error) => json_response(400, serde_json::json!({ "error": error })),
        },
        ("POST", "/find") => match parse_json::<QueryRequest>(&request.body) {
            Ok(payload) => {
                match search_text(metadata_store, &payload.query, payload.limit.unwrap_or(10)).await
                {
                    Ok(results) => json_response(200, serde_json::json!({ "results": results })),
                    Err(error) => {
                        json_response(500, serde_json::json!({ "error": error.to_string() }))
                    }
                }
            }
            Err(error) => json_response(400, serde_json::json!({ "error": error })),
        },
        ("POST", "/explain") => match parse_json::<QueryRequest>(&request.body) {
            Ok(payload) => match crate::llm::ask_local(&payload.query, metadata_store, graph_store)
                .await
            {
                Ok(answer) => json_response(200, serde_json::json!(answer)),
                Err(error) => json_response(500, serde_json::json!({ "error": error.to_string() })),
            },
            Err(error) => json_response(400, serde_json::json!({ "error": error })),
        },
        ("POST", "/where") | ("POST", "/trace") | ("POST", "/impact") => {
            match parse_json::<TargetRequest>(&request.body) {
                Ok(payload) => match graph_store
                    .get_graph_context(&payload.target, payload.limit.unwrap_or(24))
                {
                    Ok(context) => json_response(200, serde_json::json!({ "context": context })),
                    Err(error) => {
                        json_response(500, serde_json::json!({ "error": error.to_string() }))
                    }
                },
                Err(error) => json_response(400, serde_json::json!({ "error": error })),
            }
        }
        ("POST", "/index") => match parse_json::<PathRequest>(&request.body) {
            Ok(payload) => {
                match crate::indexer::index_path(&payload.path, metadata_store, graph_store).await {
                    Ok(summary) => json_response(200, serde_json::json!(summary)),
                    Err(error) => {
                        json_response(500, serde_json::json!({ "error": error.to_string() }))
                    }
                }
            }
            Err(error) => json_response(400, serde_json::json!({ "error": error })),
        },
        _ => json_response(404, serde_json::json!({ "error": "not found" })),
    }
}

fn parse_json<T: for<'de> Deserialize<'de>>(body: &str) -> Result<T, String> {
    serde_json::from_str(body).map_err(|error| error.to_string())
}

fn json_response(status: u16, value: serde_json::Value) -> String {
    let body = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };

    format!(
        "HTTP/1.1 {status} {status_text}\r\ncontent-type: application/json\r\ncontent-length: {}\r\naccess-control-allow-origin: http://localhost:1420\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[cfg(test)]
mod tests {
    use super::json_response;

    #[test]
    fn response_is_local_json_http() {
        let response = json_response(200, serde_json::json!({ "ok": true }));

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("content-type: application/json"));
        assert!(response.ends_with("{\"ok\":true}"));
    }
}
