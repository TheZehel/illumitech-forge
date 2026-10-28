use anyhow::{Result, bail, ensure};
use serde_json::{Value, json};
use std::time::Duration;

pub struct MemoryClient {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    token: Option<String>,
    session: Option<String>,
}
impl MemoryClient {
    pub fn new(endpoint: &str, token_env: Option<&str>) -> Result<Self> {
        let endpoint = reqwest::Url::parse(endpoint)?;
        ensure!(
            endpoint.username().is_empty()
                && endpoint.password().is_none()
                && endpoint.query().is_none(),
            "credentials/query not allowed in MCP URL"
        );
        ensure!(
            endpoint.scheme() == "https"
                || (endpoint.scheme() == "http"
                    && matches!(
                        endpoint.host_str(),
                        Some("127.0.0.1" | "localhost" | "[::1]")
                    )),
            "MCP requires TLS except loopback"
        );
        let token = token_env
            .map(std::env::var)
            .transpose()
            .map_err(|_| anyhow::anyhow!("MCP token environment variable unavailable"))?;
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            endpoint,
            token,
            session: None,
        })
    }
    async fn rpc(&mut self, id: u64, method: &str, params: Value) -> Result<Value> {
        let mut req = self
            .client
            .post(self.endpoint.clone())
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2024-11-05")
            .json(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        if let Some(session) = &self.session {
            req = req.header("Mcp-Session-Id", session);
        }
        let mut response = req.send().await?.error_for_status()?;
        if let Some(s) = response.headers().get("Mcp-Session-Id") {
            self.session = Some(s.to_str()?.into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                bytes.len() + chunk.len() <= 262144,
                "MCP response exceeds 256 KiB"
            );
            bytes.extend(chunk);
        }
        let text = std::str::from_utf8(&bytes)?;
        let v: Value = if let Ok(v) = serde_json::from_str(text) {
            v
        } else {
            text.lines()
                .filter_map(|l| l.strip_prefix("data:"))
                .find_map(|l| {
                    serde_json::from_str::<Value>(l.trim())
                        .ok()
                        .filter(|v| v["id"] == id)
                })
                .ok_or_else(|| anyhow::anyhow!("invalid MCP event stream"))?
        };
        ensure!(
            v["id"] == id && v["error"].is_null(),
            "MCP request failed (no unscoped fallback)"
        );
        ensure!(v["result"]["isError"] != true, "MCP tool returned an error");
        Ok(v["result"].clone())
    }
    pub async fn initialize(&mut self) -> Result<Value> {
        let info=self.rpc(1,"initialize",json!({"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"illumitech-forge","version":env!("CARGO_PKG_VERSION")}})).await?;
        let mut req = self
            .client
            .post(self.endpoint.clone())
            .header("Accept", "application/json, text/event-stream")
            .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        if let Some(s) = &self.session {
            req = req.header("Mcp-Session-Id", s);
        }
        req.send().await?.error_for_status()?;
        let tools = self.rpc(2, "tools/list", json!({})).await?;
        for name in ["memory_query", "memory_briefing"] {
            ensure!(
                tools["tools"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|t| t["name"] == name)),
                "MCP missing required tool {name}"
            );
        }
        Ok(info["serverInfo"].clone())
    }
    async fn call(&mut self, name: &str, args: Value) -> Result<Value> {
        let result = self
            .rpc(3, "tools/call", json!({"name":name,"arguments":args}))
            .await?;
        let content = result["content"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing MCP content"))?;
        for c in content {
            if let Some(t) = c["text"].as_str()
                && let Ok(v) = serde_json::from_str(t)
            {
                return Ok(v);
            }
        }
        bail!("MCP payload was not structured JSON")
    }
    pub async fn doctor(&mut self, workspace: &str, project: &str) -> Result<Value> {
        let info = self.initialize().await?;
        let b = self
            .call(
                "memory_briefing",
                json!({"workspace":workspace,"project":project}),
            )
            .await?;
        // ai-memory exposes resolved scope in briefing; absence is not proof of isolation.
        let scope = b.get("scope").unwrap_or(&b);
        ensure!(
            scope["workspace"] == workspace && scope["project"] == project,
            "memory scope could not be verified"
        );
        Ok(json!({"status":"passed","server":info,"workspace":workspace,"project":project}))
    }
    pub async fn query(&mut self, workspace: &str, project: &str, query: &str) -> Result<Value> {
        let result = self
            .call(
                "memory_query",
                json!({"workspace":workspace,"project":project,"query":query,"limit":5}),
            )
            .await?;
        let mut hits = result["hits"].as_array().cloned().unwrap_or_default();
        hits.truncate(5);
        for hit in &mut hits {
            if let Some(s) = hit["snippet"].as_str() {
                hit["snippet"] = Value::String(s.chars().take(1600).collect());
            }
        }
        Ok(
            json!({"workspace":workspace,"project":project,"hits":hits,"trust":"historical_context_only"}),
        )
    }
}
