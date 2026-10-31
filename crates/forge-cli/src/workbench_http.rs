//! Bounded HTTP/1.1 transport: exact loopback Host, capability bootstrap, cookie + CSRF.
use crate::workbench::{Action, App, approval_choices};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Semaphore,
};
include!(concat!(env!("OUT_DIR"), "/workbench_assets.rs"));
struct Request {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}
async fn read(stream: &mut TcpStream) -> Result<Request> {
    let mut data = Vec::new();
    let end;
    loop {
        let mut chunk = [0; 2048];
        let n = stream.read(&mut chunk).await?;
        ensure!(n > 0, "incomplete request");
        data.extend_from_slice(&chunk[..n]);
        if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            ensure!(i < 16384, "headers too large");
            end = i + 4;
            break;
        }
        ensure!(data.len() < 16384, "headers too large");
    }
    let head = std::str::from_utf8(&data[..end])?;
    let mut lines = head.split("\r\n");
    let first: Vec<_> = lines.next().unwrap_or("").split_whitespace().collect();
    ensure!(
        first.len() == 3 && first[2] == "HTTP/1.1",
        "invalid request"
    );
    let method = first[0].to_owned();
    let path = first[1].to_owned();
    let mut headers = BTreeMap::new();
    for line in lines.take_while(|l| !l.is_empty()) {
        let (k, v) = line.split_once(':').context("invalid header")?;
        ensure!(
            headers
                .insert(k.to_ascii_lowercase(), v.trim().to_owned())
                .is_none(),
            "duplicate header"
        );
    }
    ensure!(
        !headers.contains_key("transfer-encoding"),
        "chunked requests unsupported"
    );
    let size = headers
        .get("content-length")
        .map(|s| s.parse::<usize>())
        .transpose()?
        .unwrap_or(0);
    ensure!(size <= 65536, "body too large");
    while data.len() < end + size {
        let mut chunk = [0; 4096];
        let n = stream.read(&mut chunk).await?;
        ensure!(n > 0, "incomplete body");
        data.extend_from_slice(&chunk[..n]);
    }
    ensure!(data.len() == end + size, "pipelining unsupported");
    Ok(Request {
        method,
        path,
        headers,
        body: data[end..].to_vec(),
    })
}
fn response(code: u16, ty: &str, body: Vec<u8>, extra: &str) -> Vec<u8> {
    let mut r=format!("HTTP/1.1 {code} {}\r\nContent-Type: {ty}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'self'; connect-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'\r\n{extra}\r\n",if code==200{"OK"}else{"Error"},body.len()).into_bytes();
    r.extend(body);
    r
}
fn json_response(code: u16, mut v: Value) -> Vec<u8> {
    v = crate::compatibility::envelope(v);
    v["api_version"] = json!(1);
    if code >= 400 {
        v["error_code"] = json!(match code {
            403 => "request_denied",
            404 => "not_found",
            _ => "operation_conflict",
        });
    }
    response(
        code,
        "application/json; charset=utf-8",
        serde_json::to_vec(&v).unwrap_or_default(),
        "",
    )
}
fn origin_ok(r: &Request, origin: &str) -> bool {
    r.headers
        .get("host")
        .is_some_and(|h| origin == format!("http://{h}"))
        && r.headers.get("origin").is_none_or(|v| v == origin)
        && r.headers
            .get("sec-fetch-site")
            .is_none_or(|v| ["same-origin", "none"].contains(&v.as_str()))
}
async fn handle(r: Request, app: Arc<App>, origin: &str, token: &str) -> Result<Vec<u8>> {
    ensure!(origin_ok(&r, origin), "origin forbidden");
    if r.path == "/api/health" && r.method == "GET" {
        return Ok(json_response(
            200,
            json!({"status":"alive","acceptance":"not_evaluated"}),
        ));
    }
    if r.path == "/api/connect" && r.method == "POST" {
        ensure!(
            r.headers.get("origin").is_some_and(|o| o == origin)
                && r.headers.get("x-forge-token").is_some_and(|t| t == token),
            "local capability required"
        );
        return Ok(response(
            200,
            "application/json",
            serde_json::to_vec(&crate::compatibility::envelope(
                json!({"api_version":1,"connected":true}),
            ))?,
            &format!("Set-Cookie: forge_session={token}; HttpOnly; SameSite=Strict; Path=/\r\n"),
        ));
    }
    if r.path.starts_with("/api/") {
        ensure!(
            r.headers.get("cookie").is_some_and(|c| c
                .split(';')
                .any(|p| p.trim() == format!("forge_session={token}")))
                && r.headers.get("x-forge-csrf").is_some_and(|t| t == token),
            "local session and CSRF required"
        );
        if r.method == "GET" && r.path == "/api/forge" {
            return Ok(match app.forge() {
                Ok(v) => json_response(200, v),
                Err(_) => json_response(
                    409,
                    json!({"error":"External manifest/state unavailable or incompatible; no acceptance inferred"}),
                ),
            });
        }
        if r.method == "GET" && r.path == "/api/state" {
            let mut state = app.snapshot().await;
            if let Some(a) = state["approvals"].as_array_mut() {
                for e in a {
                    e["choices"] = json!(approval_choices(e));
                }
            }
            return Ok(json_response(200, state));
        }
        if r.method == "POST" {
            ensure!(
                r.headers.get("origin").is_some_and(|o| o == origin),
                "Origin required for controls"
            );
            let route = r.path.strip_prefix("/api/").context("route")?;
            if route.starts_with("forge-") {
                let action: crate::operational::ForgeAction = serde_json::from_slice(&r.body)?;
                return Ok(match app.forge_action(route, action).await {
                    Ok(v) => json_response(200, v),
                    Err(e) => json_response(
                        409,
                        json!({"error":forge_adapters::process::redact(&e.to_string()),"replayed":false}),
                    ),
                });
            }
            ensure!(
                [
                    "sessions",
                    "resume",
                    "turn",
                    "interrupt",
                    "approval",
                    "reconnect",
                    "feature-preview",
                    "feature-apply",
                    "planning-preview",
                    "planning-propose"
                ]
                .contains(&route),
                "unknown control"
            );
            let action: Action = serde_json::from_slice(&r.body)?;
            return Ok(match app.action(route, action).await {
                Ok(v) => json_response(200, v),
                Err(e) => json_response(
                    409,
                    json!({"error":forge_adapters::process::redact(&e.to_string()),"replayed":false}),
                ),
            });
        }
        return Ok(json_response(404, json!({"error":"unknown route"})));
    }
    ensure!(r.method == "GET" && r.body.is_empty(), "GET required");
    let path = r.path.split('?').next().unwrap_or("");
    if let Some((ty, bytes)) = asset(if path == "/" { "/index.html" } else { path }) {
        return Ok(response(200, ty, bytes.to_vec(), ""));
    }
    Ok(json_response(
        404,
        json!({"error":"asset not found; prepare embedded UI with npm run build"}),
    ))
}
pub async fn serve(app: Arc<App>, port: u16) -> Result<Value> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let origin = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let mut entropy = [0u8; 32];
    getrandom::fill(&mut entropy)
        .map_err(|e| anyhow::anyhow!("secure capability generation failed: {e}"))?;
    let token: String = entropy.iter().map(|b| format!("{b:02x}")).collect();
    println!(
        "{}",
        crate::compatibility::envelope(
            json!({"api_version":1,"event":"workbench.started","url":format!("{origin}/#session={token}"),"pid":std::process::id()})
        )
    );
    let permits = Arc::new(Semaphore::new(32));
    let mut connections = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            _=shutdown_signal()=>break,
            Some(_)=connections.join_next(),if !connections.is_empty()=>{},
            accepted=listener.accept()=>{let(stream,peer)=accepted?;if !peer.ip().is_loopback(){continue;}let Ok(permit)=permits.clone().try_acquire_owned()else{continue};let(app,origin,token)=(app.clone(),origin.clone(),token.clone());connections.spawn(async move{let _permit=permit;let mut stream=stream;let _=tokio::time::timeout(Duration::from_secs(180),async{
                let result=match read(&mut stream).await{Ok(r)=>handle(r,app,&origin,&token).await,Err(e)=>Err(e)};
                let bytes=result.unwrap_or_else(|_|json_response(403,json!({"error":"Request denied: invalid origin, capability, framing or authorization"})));
                let _=stream.write_all(&bytes).await;let _=stream.shutdown().await;
            }).await;});}
        }
    }
    connections.abort_all();
    while connections.join_next().await.is_some() {}
    Ok(json!({"status":"stopped"}))
}
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        if let Ok(mut terminate) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! { _=tokio::signal::ctrl_c()=>{}, _=terminate.recv()=>{} }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_envelopes_have_versioned_machine_codes() {
        for (code, expected) in [
            (403, "request_denied"),
            (404, "not_found"),
            (409, "operation_conflict"),
        ] {
            let bytes = json_response(code, json!({"error":"safe diagnostic","replayed":false}));
            let split = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
            let body: Value = serde_json::from_slice(&bytes[split..]).unwrap();
            assert_eq!(body["schema_version"], 1);
            assert_eq!(body["api_version"], 1);
            assert_eq!(body["error_code"], expected);
            assert_eq!(body["replayed"], false);
        }
    }
    #[tokio::test]
    async fn authenticated_cookie_alone_never_authorizes_project_reads_or_controls() {
        let t = tempfile::tempdir().unwrap();
        let base = t.path().canonicalize().unwrap();
        let root = base.join("project");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let app = App::new(&root, &base.join("state"), None, None).unwrap();
        let origin = "http://127.0.0.1:1";
        for (method, path, csrf, request_origin, token) in [
            ("GET", "/api/state", None, None, "secret"),
            ("POST", "/api/reconnect", Some("secret"), None, "secret"),
            ("POST", "/api/connect", None, Some(origin), "wrong"),
            (
                "POST",
                "/api/connect",
                None,
                Some("https://evil.invalid"),
                "secret",
            ),
        ] {
            let mut headers = BTreeMap::from([
                ("host".into(), "127.0.0.1:1".into()),
                ("cookie".into(), "forge_session=secret".into()),
                ("x-forge-token".into(), token.into()),
            ]);
            if let Some(v) = csrf {
                headers.insert("x-forge-csrf".into(), v.into());
            }
            if let Some(v) = request_origin {
                headers.insert("origin".into(), v.into());
            }
            let r = Request {
                method: method.into(),
                path: path.into(),
                headers,
                body: b"{}".to_vec(),
            };
            assert!(handle(r, app.clone(), origin, "secret").await.is_err());
        }
        // Requests denied at the HTTP boundary must not reserve or submit an operation.
        assert!(app.reserve("not-submitted").is_ok());
    }
    #[test]
    fn standalone_package_embeds_all_referenced_ui_assets() {
        let (mime, bytes) = asset("/index.html").expect("packaged index");
        assert!(mime.starts_with("text/html"));
        let html = std::str::from_utf8(bytes).unwrap();
        for part in html.split('"').filter(|s| s.starts_with("/assets/")) {
            assert!(asset(part).is_some(), "missing embedded asset {part}");
        }
        assert!(asset("/logo.webp").is_some());
        assert!(
            std::str::from_utf8(asset("/THIRD_PARTY_LICENSES.txt").unwrap().1)
                .unwrap()
                .contains("scheduler")
        );
    }
    #[test]
    fn hostile_origin_and_host_are_rejected() {
        let mut r = Request {
            method: "GET".into(),
            path: "/api/state".into(),
            headers: BTreeMap::from([("host".into(), "127.0.0.1:1".into())]),
            body: vec![],
        };
        assert!(origin_ok(&r, "http://127.0.0.1:1"));
        r.headers
            .insert("origin".into(), "https://evil.invalid".into());
        assert!(!origin_ok(&r, "http://127.0.0.1:1"));
        r.headers.remove("origin");
        r.headers.insert("host".into(), "evil.invalid:1".into());
        assert!(!origin_ok(&r, "http://127.0.0.1:1"));
    }
}
