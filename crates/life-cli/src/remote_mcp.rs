//! A remote MCP server over HTTP, bound to the loopback interface by
//! default and guarded by a bearer token.
//!
//! The server reuses the same generated tool catalog as the stdio transport,
//! so a remote agent sees exactly the same tools with the same schemas.
use life_core::{LifeError, Result, Store};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
};

const MAX_BODY: usize = 2 * 1024 * 1024;

fn respond(status: &str, body: Value) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.to_string().len(),
        body
    )
}

fn read_request(stream: &mut TcpStream) -> Result<(String, String, String)> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut headers = String::new();
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        if line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
            if name.trim().eq_ignore_ascii_case("authorization") {
                headers.push_str(value.trim());
            }
        }
    }
    if length > MAX_BODY {
        return Err(LifeError::Invalid("Request body is too large".into()));
    }
    let mut body = vec![0u8; length];
    if length > 0 {
        reader.read_exact(&mut body)?;
    }
    Ok((
        request_line,
        headers,
        String::from_utf8_lossy(&body).to_string(),
    ))
}

/// Handle one JSON-RPC request and return the JSON-RPC reply.
pub fn handle(store: &mut Store, client: &str, request: &Value) -> Value {
    let id = request.get("id").cloned();
    let method = request["method"].as_str().unwrap_or("");
    if id.is_none() {
        return json!({"jsonrpc":"2.0","error":{"code":-32600,"message":"Missing request id"}});
    }
    match method {
        "initialize" => {
            json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":"2025-03-26","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"life-manager-remote","version":"0.1.0"}}})
        }
        "ping" => json!({"jsonrpc":"2.0","id":id,"result":{}}),
        "tools/list" => {
            json!({"jsonrpc":"2.0","id":id,"result":{"tools":crate::action_spec::tools()}})
        }
        "tools/call" => {
            let name = request["params"]["name"].as_str().unwrap_or("");
            let Some(action) = crate::action_spec::action_for_tool(name) else {
                return json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":format!("Unknown tool {name}")}});
            };
            let mut payload = request["params"]["arguments"].clone();
            if !payload.is_object() {
                payload = json!({});
            }
            if let Err(message) = crate::action_spec::validate(action, &payload) {
                return json!({"jsonrpc":"2.0","id":id,"result":{"isError":true,"content":[{"type":"text","text":json!({"error":{"code":"VALIDATION_ERROR","message":message}}).to_string()}]}});
            }
            payload["actor"] = json!(format!("remote:{client}"));
            // A remote write is recorded in the security audit, so the owner
            // can see what an outside agent changed.
            let is_write = action.ends_with(".create")
                || action.ends_with(".update")
                || action.ends_with(".delete")
                || action.ends_with(".add")
                || action.ends_with(".apply")
                || action.ends_with(".set")
                || action.ends_with(".log")
                || action.ends_with(".run")
                || action.ends_with(".complete");
            if is_write {
                let _ = store.call(
                    "security.audit.log",
                    &json!({
                        "event":"remote.mcp.write",
                        "actor":format!("remote:{client}"),
                        "detail":action
                    }),
                );
            }
            match store.call(action, &payload) {
                Ok(value) => {
                    json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":value.to_string()}]}})
                }
                Err(error) => {
                    json!({"jsonrpc":"2.0","id":id,"result":{"isError":true,"content":[{"type":"text","text":json!({"error":{"code":life_core::error_code(&error),"message":error.to_string()}}).to_string()}]}})
                }
            }
        }
        _ => json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Method not found"}}),
    }
}

/// Serve the remote MCP protocol until the process is stopped.
pub fn serve(store: Store, host: &str, port: u16) -> Result<()> {
    // Port zero asks the operating system for a free port.
    let listener = TcpListener::bind((host, port))?;
    let bound = listener.local_addr()?;
    println!(
        "{}",
        json!({"ok":true,"data":{"listening":format!("http://{bound}"),"host":host,"port":port,"authentication":"bearer token","hint":"The first request needs a token from remote_clients.create."}})
    );
    let mut store = store;
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };
        let Ok((request_line, authorization, body)) = read_request(&mut stream) else {
            let _ = stream.write_all(
                respond("400 Bad Request", json!({"error":"malformed request"})).as_bytes(),
            );
            continue;
        };
        let path = request_line
            .split_whitespace()
            .nth(1)
            .unwrap_or("/")
            .to_string();
        if path == "/health" {
            let _ = stream.write_all(
                respond("200 OK", json!({"ok":true,"service":"life-manager"})).as_bytes(),
            );
            continue;
        }
        if path != "/mcp" {
            let _ = stream
                .write_all(respond("404 Not Found", json!({"error":"unknown path"})).as_bytes());
            continue;
        }
        let token = authorization
            .strip_prefix("Bearer ")
            .or_else(|| authorization.strip_prefix("bearer "))
            .unwrap_or("")
            .trim();
        if token.is_empty() {
            let _ = stream.write_all(
                respond(
                    "401 Unauthorized",
                    json!({"error":{"code":"UNAUTHORIZED","message":"A bearer token is required"}}),
                )
                .as_bytes(),
            );
            continue;
        }
        let authorised = match store.remote_client_authorised(token) {
            Ok(Some(name)) => name,
            Ok(None) => {
                let _ = stream.write_all(
                    respond(
                        "403 Forbidden",
                        json!({"error":{"code":"FORBIDDEN","message":"That token is not valid or has been revoked"}}),
                    )
                    .as_bytes(),
                );
                continue;
            }
            Err(error) => {
                let _ = stream.write_all(
                    respond(
                        "500 Internal Server Error",
                        json!({"error":{"code":life_core::error_code(&error),"message":error.to_string()}}),
                    )
                    .as_bytes(),
                );
                continue;
            }
        };
        let request: Value = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(error) => {
                let _ = stream.write_all(
                    respond(
                        "400 Bad Request",
                        json!({"error":{"code":"INVALID_JSON","message":error.to_string()}}),
                    )
                    .as_bytes(),
                );
                continue;
            }
        };
        let reply = handle(&mut store, &authorised, &request);
        let _ = stream.write_all(respond("200 OK", reply).as_bytes());
        let _ = stream.flush();
    }
    Ok(())
}
