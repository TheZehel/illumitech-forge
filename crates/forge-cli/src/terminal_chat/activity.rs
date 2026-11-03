//! Display-only projection of native activities. No tool output grants acceptance.
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}
fn bounded(s: &str) -> String {
    if s.len() <= 24000 {
        return s.into();
    }
    let end = s.floor_char_boundary(24000);
    format!("{}\n… Exibição limitada a 24000 bytes.", &s[..end])
}
pub(super) fn items(events: &VecDeque<Value>) -> BTreeMap<String, Value> {
    let mut items = BTreeMap::<String, Value>::new();
    for event in events {
        let p = &event["params"];
        let method = text(&event["method"]);
        if matches!(method, "item/started" | "item/completed") {
            let id = text(&p["item"]["id"]);
            if id.is_empty() {
                continue;
            }
            let mut value = items.remove(id).unwrap_or_else(|| serde_json::json!({}));
            if let Some(new) = p["item"].as_object() {
                for (key, v) in new {
                    value[key] = v.clone();
                }
            }
            if value["status"].is_null() && method == "item/started" {
                value["status"] = "inProgress".into();
            } else if method == "item/completed" && p["item"]["status"].is_null() {
                value["status"] = Value::Null;
            }
            items.insert(id.into(), value);
        } else if matches!(
            method,
            "item/commandExecution/outputDelta"
                | "item/fileChange/outputDelta"
                | "item/mcpToolCall/progress"
        ) {
            let id = text(&p["itemId"]);
            if id.is_empty() {
                continue;
            }
            let kind = if method.contains("commandExecution") {
                "commandExecution"
            } else if method.contains("fileChange") {
                "fileChange"
            } else {
                "mcpToolCall"
            };
            let item = items
                .entry(id.into())
                .or_insert_with(|| serde_json::json!({"id":id,"type":kind,"status":"inProgress"}));
            let (field, delta) = if kind == "mcpToolCall" {
                ("progress", text(&p["message"]))
            } else {
                ("aggregatedOutput", text(&p["delta"]))
            };
            item[field] = bounded(&format!("{}{delta}", text(&item[field]))).into();
        }
    }
    items
}
pub(super) fn counts(diff: &str) -> Option<(usize, usize)> {
    let (mut hunk, mut observed, mut add, mut remove) = (false, false, 0, 0);
    for line in diff.lines() {
        if line.starts_with("diff ") || line.starts_with("*** ") {
            hunk = false;
        }
        if line.starts_with("@@ ") {
            hunk = true;
            observed = true;
        } else if hunk && line.starts_with('+') {
            add += 1;
        } else if hunk && line.starts_with('-') {
            remove += 1;
        }
    }
    observed.then_some((add, remove))
}
pub(super) fn entry(item: &Value) -> String {
    let mut sections = Vec::new();
    let mut add = |label: &str, value: &Value| {
        if !value.is_null() {
            let value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| serde_json::to_string_pretty(value).unwrap_or_default());
            if !value.is_empty() {
                sections.push(format!("{label}\n{}", bounded(&value)));
            }
        }
    };
    let title = match text(&item["type"]) {
        "mcpToolCall" | "dynamicToolCall" => {
            add("Argumentos", &item["arguments"]);
            add("Resultado", &item["result"]);
            add("Erro", &item["error"]);
            add("Progresso", &item["progress"]);
            add("Conteúdo retornado", &item["contentItems"]);
            let name = [text(&item["server"]), text(&item["tool"])]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(".");
            format!(
                "Chamada · {}",
                if name.is_empty() { "Ferramenta" } else { &name }
            )
        }
        "commandExecution" => {
            add("Comando", &item["command"]);
            add("Diretório", &item["cwd"]);
            add("Código de saída", &item["exitCode"]);
            add("Saída", &item["aggregatedOutput"]);
            format!(
                "Comando · {}",
                bounded(
                    text(&item["command"])
                        .lines()
                        .next()
                        .unwrap_or("Execução nativa")
                )
            )
        }
        "fileChange" => {
            let files = item["changes"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .filter(|f| f.is_object())
                .collect::<Vec<_>>();
            let stats = files
                .iter()
                .map(|f| counts(text(&f["diff"])))
                .collect::<Option<Vec<_>>>();
            let count = stats
                .filter(|_| !files.is_empty())
                .map(|s| s.into_iter().fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1)));
            for file in &files {
                let suffix = counts(text(&file["diff"]))
                    .map(|(a, r)| format!(" (+{a} −{r})"))
                    .unwrap_or_default();
                add(&format!("{}{suffix}", text(&file["path"])), &file["diff"]);
            }
            add("Saída", &item["aggregatedOutput"]);
            let suffix = count
                .map(|(a, r)| format!(" (+{a} −{r}; hunks recebidos)"))
                .unwrap_or_default();
            let total = item["forgeOriginalFileCount"]
                .as_u64()
                .filter(|n| *n > files.len() as u64)
                .map(|n| format!("{} de {n}", files.len()))
                .unwrap_or_else(|| files.len().to_string());
            format!("Alterações · {total} arquivo(s){suffix}")
        }
        "webSearch" => {
            add("Consulta", &item["query"]);
            add("Ação", &item["action"]);
            "Pesquisa".into()
        }
        _ => {
            add("Dados informados pelo cliente", item);
            "Atividade do cliente".into()
        }
    };
    let status = if !item["error"].is_null()
        || item["result"]["isError"] == true
        || item["success"] == false
        || item["exitCode"].as_i64().is_some_and(|c| c != 0)
    {
        "failed"
    } else {
        text(&item["status"])
    };
    let truncated = item["forgeDisplayTruncated"] == true;
    format!(
        "{}{}|{status}\n{}{}",
        title
            .chars()
            .take(180)
            .collect::<String>()
            .replace('|', "¦")
            .replace('\n', " "),
        if truncated { " · parcial" } else { "" },
        if truncated {
            "Exibição parcial; conteúdo e contagens podem estar incompletos.\n"
        } else {
            ""
        },
        sections.join("\n\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn only_received_hunks_count() {
        assert_eq!(
            counts("--- a\n+++ b\n@@ -1 +1,2 @@\n-old\n+new\n+++ literal\n"),
            Some((2, 1))
        );
        assert_eq!(counts("+++ b\n-new\n+new"), None);
    }
    #[test]
    fn mcp_payload_and_error_are_preserved() {
        let value = entry(
            &json!({"type":"mcpToolCall","server":"logs","tool":"query","arguments":{"limit":4},"result":{"isError":true,"content":[{"text":"falha"}]},"status":"completed"}),
        );
        assert!(value.starts_with("Chamada · logs.query|failed"));
        assert!(value.contains("Argumentos\n"));
        assert!(value.contains("Resultado\n"));
    }
}
