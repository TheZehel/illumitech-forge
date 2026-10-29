//! Bounded, read-only projection of external operational state. No execution endpoints.
use anyhow::{Result, bail, ensure};
use forge_adapters::process::redact;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const MAX_BODY: usize = 1024 * 1024;
const PAGE: usize = 100;
const EVENTS: usize = 200;

pub async fn serve(state: &Path, port: u16) -> Result<Value> {
    external(state)?;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    eprintln!(
        "Dashboard somente leitura: http://127.0.0.1:{}/ — Ctrl+C para encerrar",
        listener.local_addr()?.port()
    );
    tokio::select! {
        result = run(listener, state.join("state.sqlite")) => result?,
        result = tokio::signal::ctrl_c() => result?,
    }
    Ok(json!({"status":"stopped"}))
}

fn external(path: &Path) -> Result<()> {
    let absolute = std::path::absolute(path)?;
    ensure!(
        !absolute
            .components()
            .any(|c| c == std::path::Component::ParentDir),
        "use um diretório externo sem componentes '..'"
    );
    let existing = absolute
        .ancestors()
        .find(|p| p.exists())
        .ok_or_else(|| anyhow::anyhow!("diretório indisponível"))?
        .canonicalize()?;
    ensure!(
        !existing.ancestors().any(|p| p.join(".git").exists()),
        "o estado deve ficar fora de repositórios Git"
    );
    let db = path.join("state.sqlite");
    if db.exists() {
        ensure!(
            !db.canonicalize()?
                .ancestors()
                .any(|p| p.join(".git").exists()),
            "o banco deve ficar fora de repositórios Git"
        );
    }
    Ok(())
}

async fn run(listener: TcpListener, db: PathBuf) -> Result<()> {
    let port = listener.local_addr()?.port();
    // Sequential requests bound connections and database work; each peer has a total deadline.
    loop {
        let (stream, peer) = listener.accept().await?;
        if peer.ip().is_loopback() {
            let _ =
                tokio::time::timeout(Duration::from_secs(3), connection(stream, &db, port)).await;
        }
    }
}

async fn connection(mut stream: TcpStream, db: &Path, port: u16) -> Result<()> {
    let mut request = Vec::new();
    let response = loop {
        let mut chunk = [0; 1024];
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(());
        }
        request.extend_from_slice(&chunk[..n]);
        if request.len() > 8192 {
            break reply(431, "Cabeçalhos excessivos".into());
        }
        if request.windows(4).any(|w| w == b"\r\n\r\n") {
            break match std::str::from_utf8(&request) {
                Ok(request) => handle(request, db, port),
                Err(_) => reply(400, "Requisição inválida".into()),
            };
        }
    };
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;
    Ok(())
}

fn reply(code: u16, body: String) -> String {
    let (code, body) = if body.len() > MAX_BODY {
        (503, "Visualização excede o limite".into())
    } else {
        (code, body)
    };
    let reason = match code {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        431 => "Request Header Fields Too Large",
        _ => "Service Unavailable",
    };
    format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\r\n{}\r\n{body}",
        body.len(),
        if code == 405 { "Allow: GET\r\n" } else { "" }
    )
}

fn handle(request: &str, db: &Path, port: u16) -> String {
    let mut lines = request.split("\r\n");
    let first: Vec<_> = lines
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    if first.len() != 3 || first[2] != "HTTP/1.1" {
        return reply(400, "Requisição inválida".into());
    }
    let mut headers = BTreeMap::new();
    for line in lines.take_while(|s| !s.is_empty()) {
        let Some((key, value)) = line.split_once(':') else {
            return reply(400, "Cabeçalho inválido".into());
        };
        if headers
            .insert(key.to_ascii_lowercase(), value.trim())
            .is_some()
        {
            return reply(400, "Cabeçalho duplicado".into());
        }
    }
    let host = format!("127.0.0.1:{port}");
    let origin = format!("http://{host}");
    if headers.get("host") != Some(&host.as_str())
        || headers.get("origin").is_some_and(|v| *v != origin)
        || headers
            .get("sec-fetch-site")
            .is_some_and(|v| !["same-origin", "none"].contains(v))
    {
        return reply(403, "Origem não permitida".into());
    }
    if first[0] != "GET" {
        return reply(405, "Somente leitura: use GET".into());
    }
    if headers.contains_key("transfer-encoding")
        || headers.get("content-length").is_some_and(|v| *v != "0")
    {
        return reply(400, "Corpo não permitido".into());
    }
    let (path, query) = first[1].split_once('?').unwrap_or((first[1], ""));
    let mut params = BTreeMap::new();
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let Some((k, v)) = pair.split_once('=') else {
            return reply(400, "Filtro inválido".into());
        };
        if !["after", "events", "task", "gate"].contains(&k)
            || v.len() > 512
            || params.insert(k, v).is_some()
        {
            return reply(400, "Filtro inválido".into());
        }
    }
    if path == "/health" {
        return reply(
            200,
            "Servidor local ativo; saúde dos gates não avaliada.".into(),
        );
    }
    let release = if path == "/" {
        None
    } else if let Some(id) = path.strip_prefix("/releases/") {
        match decode(id) {
            Some(id) => Some(id),
            None => return reply(404, "Release não encontrada".into()),
        }
    } else {
        return reply(404, "Página não encontrada".into());
    };
    match render(db, release.as_deref(), &params) {
        Ok(Some(body)) => reply(200, body),
        Ok(None) => reply(404, "Release não encontrada".into()),
        Err(_) => reply(503, "Estado indisponível. Banco incompatível, ocupado, excessivo ou inválido; tente novamente.".into()),
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn label(s: &str) -> String {
    escape(&redact(&s.chars().take(128).collect::<String>()))
}
fn encode(s: &str) -> String {
    s.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}
fn decode(s: &str) -> Option<String> {
    if s.is_empty()
        || s.len() > 512
        || !s.len().is_multiple_of(2)
        || !s.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    String::from_utf8(
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
            .collect::<Option<Vec<_>>>()?,
    )
    .ok()
}
fn status(value: &Value) -> &'static str {
    match value.as_str().unwrap_or_default() {
        "passed" => "Aprovado (passed)",
        "failed" => "Falhou (failed)",
        "blocked" => "Bloqueado (blocked)",
        "running" => "Em execução",
        "evaluating" => "Em avaliação",
        "paused" => "Pausado",
        "cancelled" => "Cancelado",
        "awaiting_approval" => "Aguardando aprovação humana",
        "not_run" | "" => "Não executado (not_run)",
        _ => "Desconhecido",
    }
}
fn shell(body: &str) -> String {
    format!(
        r##"<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Illumitech Forge · Painel local</title><style>
:root{{color-scheme:dark;font-family:system-ui,sans-serif;background:#171b20;color:#f3f4f6}}*{{box-sizing:border-box}}body{{margin:0}}header,main,footer{{max-width:1120px;margin:auto;padding:24px}}header{{border-bottom:3px solid #ff9b54}}h1{{font-size:clamp(1.6rem,5vw,2.4rem)}}h2{{color:#ffae73;margin-top:32px}}a{{color:#70e1d1;text-underline-offset:4px}}a:focus-visible{{outline:3px solid #ffae73;outline-offset:4px}}.notice{{border-left:4px solid #70e1d1;padding:16px;background:#232a32}}.table{{overflow-x:auto}}table{{border-collapse:collapse;table-layout:fixed;width:100%;margin:16px 0}}th,td{{text-align:left;padding:12px;border-bottom:1px solid #4e5966;overflow-wrap:anywhere}}th{{color:#c8d0d9}}nav{{display:flex;gap:16px;flex-wrap:wrap}}footer{{color:#c8d0d9}}.skip{{position:absolute;left:-9999px}}.skip:focus{{position:static}}@media(max-width:480px){{header,main,footer{{padding:16px}}th,td{{padding:8px}}}}
</style></head><body><a class="skip" href="#conteudo">Ir ao conteúdo</a><header><p>ILLUMITECH / FORGE</p><h1>Painel de execução local</h1><nav aria-label="Principal"><a href="/">Releases</a><a href="/health">Servidor</a></nav></header><main id="conteudo"><p class="notice">Somente leitura · Atualize a página para consultar o estado. Resultados registrados não substituem aceite operacional.</p>{body}</main><footer>Estado externo · Sem execução de agentes · Sem serviços ativados</footer></body></html>"##
    )
}

fn render(
    db: &Path,
    release: Option<&str>,
    params: &BTreeMap<&str, &str>,
) -> Result<Option<String>> {
    external(db.parent().ok_or_else(|| anyhow::anyhow!("invalid path"))?)?;
    if !db.try_exists()? {
        return Ok(if release.is_some() {
            None
        } else {
            Some(shell(
                "<h2>Nenhum estado disponível</h2><p>O banco ainda não existe. Nenhum arquivo foi criado.</p>",
            ))
        });
    }
    let conn = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(Duration::from_millis(200))?;
    let start = Instant::now();
    conn.progress_handler(
        1000,
        Some(move || start.elapsed() > Duration::from_millis(500)),
    )?;
    conn.execute_batch("PRAGMA query_only=ON; PRAGMA trusted_schema=OFF; BEGIN;")?;
    let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    ensure!(version == 1, "schema");
    let mut html = heartbeat(&conn)?;
    if let Some(id) = release {
        return detail(&conn, id, params, html).map(|v| v.map(|s| shell(&s)));
    }
    let after = params
        .get("after")
        .map(|v| decode(v).ok_or_else(|| anyhow::anyhow!("cursor")))
        .transpose()?
        .unwrap_or_default();
    let mut stmt = conn.prepare("SELECT id, CASE WHEN length(CAST(body AS BLOB))<=1048576 THEN substr(json_extract(body,'$.status'),1,40) END FROM runs WHERE id > ?1 AND length(id)<=256 ORDER BY id LIMIT 100")?;
    let rows = stmt
        .query_map([after], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    html.push_str("<h2>Releases</h2><p>Até 100 por página.</p><div class=\"table\"><table><caption>Releases registradas</caption><tr><th scope=\"col\">Release</th><th scope=\"col\">Status</th></tr>");
    for (id, state) in &rows {
        html.push_str(&format!(
            "<tr><td><a href=\"/releases/{}\">{}</a></td><td>{}</td></tr>",
            encode(id),
            label(id),
            status(&json!(state))
        ));
    }
    html.push_str("</table></div>");
    if rows.is_empty() {
        html.push_str("<p>Nenhuma release nesta página.</p>");
    }
    if rows.len() == PAGE {
        html.push_str(&format!(
            "<a href=\"/?after={}\">Próximas releases</a>",
            encode(&rows.last().unwrap().0)
        ));
    }
    Ok(Some(shell(&html)))
}

fn heartbeat(conn: &Connection) -> Result<String> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='metadata')",
        [],
        |r| r.get(0),
    )?;
    let mut text = "Heartbeat ausente; atividade não confirmada";
    if exists {
        let beat: Option<(Option<i64>, Option<bool>)> = conn.query_row("SELECT json_extract(body,'$.at_epoch'), json_extract(body,'$.running') FROM metadata WHERE key='daemon_heartbeat' AND length(body)<=4096", [], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
        if let Some((Some(at), Some(running))) = beat {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
            text = if !running {
                "Daemon parado"
            } else if (0..=30).contains(&(now - at)) {
                "Heartbeat recente (até 30 s); não comprova progresso"
            } else {
                "Heartbeat antigo ou inválido; atividade não confirmada"
            };
        }
    }
    Ok(format!("<p class=\"notice\">{text}</p>"))
}

fn detail(
    conn: &Connection,
    id: &str,
    params: &BTreeMap<&str, &str>,
    mut html: String,
) -> Result<Option<String>> {
    // Bound the stored JSON before transferring it out of SQLite. Never render the manifest,
    // memory, revision paths, evidence details or event bodies.
    let body: Option<Option<String>> = conn.query_row("SELECT CASE WHEN length(CAST(body AS BLOB))<=1048576 THEN body END FROM runs WHERE id=?1", [id], |r| r.get(0)).optional()?;
    let Some(body) = body else {
        return Ok(None);
    };
    let Some(body) = body else {
        bail!("oversized state");
    };
    let run: Value = serde_json::from_str(&body)?;
    html.push_str(&format!(
        "<h2>Release {}</h2><p>Status: {}</p>",
        label(id),
        status(&run["status"])
    ));
    let base = format!("/releases/{}", encode(id));
    let after = params
        .get("after")
        .map(|v| decode(v).ok_or_else(|| anyhow::anyhow!("cursor")))
        .transpose()?
        .unwrap_or_default();
    let tasks = run["tasks"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("tasks"))?;
    let page: Vec<_> = tasks
        .iter()
        .filter(|(key, _)| key.as_str() > after.as_str())
        .take(PAGE)
        .collect();
    html.push_str("<h2>Tarefas</h2><p>Até 100 por página. Selecione uma tarefa para inspecionar seus gates.</p><div class=\"table\"><table><caption>Tarefas e tentativas</caption><tr><th scope=\"col\">Tarefa</th><th scope=\"col\">Status</th><th scope=\"col\">Tentativas</th></tr>");
    for (key, task) in &page {
        html.push_str(&format!(
            "<tr><td><a href=\"{base}?task={}\">{}</a></td><td>{}</td><td>{}</td></tr>",
            encode(key),
            label(key),
            status(&task["status"]),
            task["attempts"].as_u64().unwrap_or(0)
        ));
    }
    html.push_str("</table></div>");
    if page.is_empty() {
        html.push_str("<p>Nenhuma tarefa nesta página.</p>");
    }
    if page.len() == PAGE {
        html.push_str(&format!(
            "<a href=\"{base}?after={}\">Próximas tarefas</a>",
            encode(page.last().unwrap().0)
        ));
    }
    html.push_str("<h2>Gates</h2>");
    if let Some(task) = params.get("task") {
        let key = decode(task).ok_or_else(|| anyhow::anyhow!("task"))?;
        let state = tasks.get(&key).ok_or_else(|| anyhow::anyhow!("task"))?;
        let offset = params
            .get("gate")
            .map(|v| v.parse::<usize>())
            .transpose()?
            .unwrap_or(0);
        html.push_str(&format!(
            "<h3>Tarefa {}</h3><p>Até 100 evidências registradas por página.</p><ul>",
            label(&key)
        ));
        let evidence = state["evidence"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("evidence"))?;
        for gate in evidence.iter().skip(offset).take(PAGE) {
            html.push_str(&format!(
                "<li>{}: {}</li>",
                label(gate["id"].as_str().unwrap_or("Gate")),
                status(&gate["outcome"])
            ));
        }
        html.push_str("</ul>");
        if evidence.is_empty() {
            html.push_str("<p>Sem evidência registrada: não executado (not_run).</p>");
        }
        if offset.saturating_add(PAGE) < evidence.len() {
            html.push_str(&format!(
                "<a href=\"{base}?task={task}&amp;gate={}\">Próximos gates</a>",
                offset + PAGE
            ));
        }
    } else {
        html.push_str(
            "<p>Selecione uma tarefa. Ausência de evidência não significa aprovação.</p>",
        );
    }
    let cursor = params
        .get("events")
        .map(|v| v.parse::<i64>())
        .transpose()?
        .unwrap_or(0);
    ensure!(cursor >= 0, "cursor");
    let mut stmt = conn.prepare("SELECT seq, substr(at,1,32), substr(kind,1,64) FROM events WHERE release_id=?1 AND seq>?2 ORDER BY seq LIMIT 200")?;
    let events = stmt
        .query_map(rusqlite::params![id, cursor], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    html.push_str("<h2>Eventos</h2><p>Até 200 por página; conteúdo bruto omitido.</p><div class=\"table\"><table><caption>Eventos registrados em UTC</caption><tr><th scope=\"col\">Sequência</th><th scope=\"col\">Data UTC</th><th scope=\"col\">Tipo</th></tr>");
    for (seq, at, kind) in &events {
        let kind = match kind.as_str() {
            "prepared" => "Preparado",
            "started" => "Iniciado",
            "stopped" => "Parado",
            "evaluated" => "Avaliado",
            "task_exited" => "Tarefa encerrada",
            "process_spawned" => "Processo iniciado",
            "control" => "Controle",
            "approval" => "Aprovação",
            "memory_blocked" => "Memória bloqueada",
            _ => "Outro evento",
        };
        let at = if at
            .bytes()
            .all(|b| b.is_ascii_digit() || b"-:TZ. ".contains(&b))
        {
            at.as_str()
        } else {
            "Data inválida"
        };
        html.push_str(&format!(
            "<tr><td>{seq}</td><td>{}</td><td>{kind}</td></tr>",
            escape(at)
        ));
    }
    html.push_str("</table></div>");
    if events.is_empty() {
        html.push_str("<p>Nenhum evento nesta página.</p>");
    }
    if events.len() == EVENTS {
        html.push_str(&format!(
            "<a href=\"{base}?events={}\">Próximos eventos</a>",
            events.last().unwrap().0
        ));
    }
    Ok(Some(html))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(path: &Path) -> Connection {
        let _ = forge_domain::Store::open(path, false).unwrap();
        Connection::open(path).unwrap()
    }
    fn insert(conn: &Connection, id: &str) {
        let tasks = json!({"task<script>": {"status":"blocked", "attempts":2, "evidence":[
            {"id":"gate<&\"", "outcome":"failed", "detail":"PRIVATE_DETAIL"},
            {"id":"waiting", "outcome":"not_run"}, {"id":"ok", "outcome":"passed"},
            {"id":"blocked", "outcome":"blocked"}
        ]}, "empty": {"status":"not_run", "attempts":0, "evidence":[]}});
        conn.execute("INSERT INTO runs VALUES(?1,?2)", rusqlite::params![id, json!({"status":"blocked", "tasks":tasks, "memory":"PRIVATE_MEMORY", "effective_manifest":{"prompt":"PRIVATE_PROMPT"}}).to_string()]).unwrap();
        conn.execute(
            "INSERT INTO events(release_id,kind,body) VALUES(?1,'started',?2)",
            rusqlite::params![id, "PRIVATE_EVENT"],
        )
        .unwrap();
    }
    async fn http(db: &Path, method: &str, path: &str, headers: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let db = db.to_owned();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            connection(stream, &db, port).await.unwrap();
        });
        let mut client = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .unwrap();
        let headers = headers.replace("PORT", &port.to_string());
        client
            .write_all(format!("{method} {path} HTTP/1.1\r\n{headers}\r\n").as_bytes())
            .await
            .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();
        server.await.unwrap();
        response
    }
    const HOST: &str = "Host: 127.0.0.1:PORT\r\n";

    #[tokio::test]
    async fn empty_corrupt_and_unknown_states_are_safe() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("missing/state.sqlite");
        let response = http(&db, "GET", "/", HOST).await;
        assert!(response.contains("200 OK") && response.contains("Nenhum estado disponível"));
        assert!(!db.parent().unwrap().exists());
        assert!(http(&db, "GET", "/health", HOST).await.contains("200 OK"));
        let db = tmp.path().join("state.sqlite");
        std::fs::write(&db, "not a database PRIVATE_SECRET").unwrap();
        let response = http(&db, "GET", "/", HOST).await;
        assert!(response.contains("503 Service Unavailable"));
        assert!(
            !response.contains("PRIVATE_SECRET")
                && !response.contains(&tmp.path().display().to_string())
        );
    }

    #[tokio::test]
    async fn http_security_and_route_boundary() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("state.sqlite");
        for headers in [
            "Host: attacker.example\r\n",
            "Host: 127.0.0.1:1\r\n",
            "",
            "Host: 127.0.0.1:PORT\r\nOrigin: https://attacker.example\r\n",
            "Host: 127.0.0.1:PORT\r\nSec-Fetch-Site: cross-site\r\n",
        ] {
            assert!(
                http(&db, "GET", "/", headers)
                    .await
                    .starts_with("HTTP/1.1 403")
            );
        }
        for method in ["POST", "DELETE", "PUT", "OPTIONS", "HEAD"] {
            let response = http(&db, method, "/", HOST).await;
            assert!(response.starts_with("HTTP/1.1 405") && response.contains("Allow: GET"));
        }
        for path in [
            "/run",
            "/files/etc/passwd",
            "/releases/../../x",
            "/releases/%27OR%201",
            "/releases/ff",
            "/releases/00ff",
            "/unknown",
        ] {
            assert!(
                http(&db, "GET", path, HOST)
                    .await
                    .starts_with("HTTP/1.1 404")
            );
        }
        for headers in [
            "Host: 127.0.0.1:PORT\r\nHost: 127.0.0.1:PORT\r\n",
            "Host: 127.0.0.1:PORT\r\nTransfer-Encoding: chunked\r\n",
            "Host: 127.0.0.1:PORT\r\nContent-Length: 1\r\n",
        ] {
            assert!(
                http(&db, "GET", "/", headers)
                    .await
                    .starts_with("HTTP/1.1 400")
            );
        }
        let oversized = format!("{HOST}X-Large: {}\r\n", "a".repeat(9000));
        assert!(
            http(&db, "GET", "/", &oversized)
                .await
                .starts_with("HTTP/1.1 431")
        );
        let response = http(&db, "GET", "/", HOST).await;
        assert!(
            response.contains("frame-ancestors 'none'")
                && response.contains("Cache-Control: no-store")
        );
        assert!(!response.contains("Access-Control-Allow") && !response.contains("<script"));
    }

    #[tokio::test]
    async fn escaped_projection_and_database_preservation() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("state.sqlite");
        let conn = fixture(&db);
        let id = "release<script>\"&";
        insert(&conn, id);
        conn.execute(
            "INSERT INTO metadata VALUES('daemon_heartbeat',?1)",
            [json!({"at_epoch":1,"running":true}).to_string()],
        )
        .unwrap();
        drop(conn);
        let before = forge_domain::digest(&std::fs::read(&db).unwrap());
        let response = http(&db, "GET", "/", HOST).await;
        assert!(response.contains("release&lt;script&gt;&quot;&amp;"));
        assert!(response.contains("Heartbeat antigo"));
        let response = http(
            &db,
            "GET",
            &format!("/releases/{}?task={}", encode(id), encode("task<script>")),
            HOST,
        )
        .await;
        for expected in [
            "200 OK",
            "gate&lt;&amp;&quot;",
            "failed",
            "passed",
            "blocked",
            "not_run",
            "Iniciado",
        ] {
            assert!(response.contains(expected), "missing {expected}");
        }
        for forbidden in [
            "<script>",
            "PRIVATE_DETAIL",
            "PRIVATE_MEMORY",
            "PRIVATE_PROMPT",
            "PRIVATE_EVENT",
        ] {
            assert!(!response.contains(forbidden));
        }
        assert_eq!(before, forge_domain::digest(&std::fs::read(&db).unwrap()));
        let response = http(
            &db,
            "GET",
            &format!("/releases/{}?task={}", encode(id), encode("empty")),
            HOST,
        )
        .await;
        assert!(response.contains("Sem evidência registrada"));
        assert!(
            http(
                &db,
                "GET",
                &format!("/releases/{}", encode("' OR 1=1 --")),
                HOST
            )
            .await
            .starts_with("HTTP/1.1 404")
        );
    }

    #[tokio::test]
    async fn bounded_pages_and_concurrent_writer() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("state.sqlite");
        let mut conn = fixture(&db);
        for i in 0..101 {
            insert(&conn, &format!("r{i:03}"));
        }
        let mut tasks = serde_json::Map::new();
        for i in 0..101 {
            tasks.insert(format!("t{i:03}"), json!({"status":"not_run", "attempts":0, "evidence":(0..101).map(|i| json!({"id":format!("g{i:03}"),"outcome":"not_run"})).collect::<Vec<_>>()}));
        }
        conn.execute(
            "UPDATE runs SET body=?1 WHERE id='r000'",
            [json!({"status":"running","tasks":tasks}).to_string()],
        )
        .unwrap();
        for _ in 0..201 {
            conn.execute("INSERT INTO events(release_id,kind,body) VALUES('r000','evaluated','PRIVATE_EVENT')", []).unwrap();
        }
        let response = http(&db, "GET", "/", HOST).await;
        assert!(response.contains(">r099</a>") && !response.contains(">r100</a>"));
        let next = http(&db, "GET", &format!("/?after={}", encode("r099")), HOST).await;
        assert!(next.contains(">r100</a>") && !next.contains(">r099</a>"));
        let path = format!("/releases/{}?task={}", encode("r000"), encode("t000"));
        let response = http(&db, "GET", &path, HOST).await;
        assert!(response.contains(">t099</a>") && !response.contains(">t100</a>"));
        assert!(response.contains("g099:") && !response.contains("g100:"));
        assert_eq!(response.matches("Avaliado</td>").count(), 199);
        assert!(response.len() < MAX_BODY);
        let next = http(
            &db,
            "GET",
            &format!("{path}&gate=100&after={}&events=300", encode("t099")),
            HOST,
        )
        .await;
        assert!(next.contains("g100:") && next.contains(">t100</a>"));
        assert_eq!(next.matches("Avaliado</td>").count(), 2);
        // An uncommitted WAL writer cannot leak partial updates into the read snapshot.
        let tx = conn.transaction().unwrap();
        tx.execute("UPDATE runs SET body='invalid JSON' WHERE id='r000'", [])
            .unwrap();
        assert!(
            http(&db, "GET", &path, HOST)
                .await
                .starts_with("HTTP/1.1 200")
        );
        tx.rollback().unwrap();
        conn.execute(
            "UPDATE runs SET body=?1 WHERE id='r000'",
            ["x".repeat(MAX_BODY + 1)],
        )
        .unwrap();
        assert!(
            http(&db, "GET", &path, HOST)
                .await
                .starts_with("HTTP/1.1 503")
        );
    }

    #[tokio::test]
    async fn locked_and_incompatible_database_fail_closed() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("state.sqlite");
        let conn = fixture(&db);
        conn.execute_batch("PRAGMA journal_mode=DELETE; BEGIN EXCLUSIVE;")
            .unwrap();
        let started = Instant::now();
        let response = http(&db, "GET", "/", HOST).await;
        assert!(response.starts_with("HTTP/1.1 503"));
        assert!(started.elapsed() < Duration::from_secs(2));
        conn.execute_batch("ROLLBACK; PRAGMA user_version=99;")
            .unwrap();
        assert!(
            http(&db, "GET", "/", HOST)
                .await
                .starts_with("HTTP/1.1 503")
        );
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            99
        );
    }

    #[test]
    fn refuses_repository_state_and_caps_responses() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        assert!(external(&tmp.path().join("state")).is_err());
        assert!(reply(200, "x".repeat(MAX_BODY + 1)).starts_with("HTTP/1.1 503"));
        assert_eq!(escape("<&>\"'"), "&lt;&amp;&gt;&quot;&#39;");
    }
}
