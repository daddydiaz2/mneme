/// Simple MCP server — handles JSON-RPC over stdio directly.
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use crate::store::db::Database;

pub async fn run_simple_stdio(db: Arc<Database>) -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();
    let mut initialized = false;

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() { continue; }

        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err = serde_json::json!({
                    "jsonrpc": "2.0", "id": null,
                    "error": {"code": -32700, "message": format!("Parse error: {}", e)}
                });
                stdout.write_all(format!("{}\n", serde_json::to_string(&err)?).as_bytes()).await?;
                stdout.flush().await?;
                continue;
            }
        };

        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = msg.get("id");

        match method {
            "initialize" => {
                let result = serde_json::json!({
                    "jsonrpc": "2.0", "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "mneme", "version": env!("CARGO_PKG_VERSION")},
                        "instructions": "Mneme MCP server — persistent memory for AI agents"
                    }
                });
                stdout.write_all(format!("{}\n", serde_json::to_string(&result)?).as_bytes()).await?;
                stdout.flush().await?;
            }
            "notifications/initialized" => {
                initialized = true;
            }
            "tools/list" => {
                if !initialized { continue; }
                let tools = crate::mcp::tools::list_tools(None);
                let tool_list: Vec<serde_json::Value> = tools.iter().map(|t| {
                    serde_json::json!({
                        "name": t.name.clone(),
                        "description": t.description.clone(),
                        "inputSchema": t.input_schema
                    })
                }).collect();
                let result = serde_json::json!({
                    "jsonrpc": "2.0", "id": id,
                    "result": {"tools": tool_list}
                });
                stdout.write_all(format!("{}\n", serde_json::to_string(&result)?).as_bytes()).await?;
                stdout.flush().await?;
            }
            "tools/call" => {
                if !initialized { continue; }
                let params = msg.get("params");
                let tool_name = params.and_then(|p| p.get("name")).and_then(|n| n.as_str()).unwrap_or("");
                let arguments = params.and_then(|p| p.get("arguments")).and_then(|a| a.as_object()).cloned();

                let args_map = arguments.unwrap_or_default();
                let project = "default".to_string();
                let result = crate::mcp::tools::execute_tool(
                    &db, tool_name, Some(args_map), &project, None, None,
                ).await;

                // Serialize the CallToolResult directly (it derives Serialize)
                let response = serde_json::json!({
                    "jsonrpc": "2.0", "id": id,
                    "result": serde_json::to_value(&result)?
                });
                stdout.write_all(format!("{}\n", serde_json::to_string(&response)?).as_bytes()).await?;
                stdout.flush().await?;
            }
            _ => {
                if id.is_some() {
                    let err = serde_json::json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": {"code": -32601, "message": format!("Method not found: {}", method)}
                    });
                    stdout.write_all(format!("{}\n", serde_json::to_string(&err)?).as_bytes()).await?;
                    stdout.flush().await?;
                }
            }
        }
    }
    Ok(())
}
