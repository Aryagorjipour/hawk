use serde_json::Value;

use super::crawl::{
    looks_like_refusal, CrawlItem, CrawlSourceRef, ExtractStatus, StructuredCrawlResult,
};
use super::error::{DomainError, DomainResult};

pub fn parse_structured_result(raw: &str) -> DomainResult<StructuredCrawlResult> {
    let trimmed = raw.trim();
    let json_str = extract_json_blob(trimmed)?;

    let value: Value = serde_json::from_str(json_str)
        .map_err(|e| DomainError::Parse(format!("LLM JSON parse failed: {e}")))?;

    let status = match value.get("status").and_then(|v| v.as_str()).unwrap_or("ok") {
        "ok" => ExtractStatus::Ok,
        "partial" => ExtractStatus::Partial,
        "unable" => ExtractStatus::Unable,
        other => {
            return Err(DomainError::Parse(format!(
                "unknown extract status: {other}"
            )))
        }
    };

    let language = value
        .get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("en")
        .to_string();
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let summary = value
        .get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if looks_like_refusal(&summary) && status != ExtractStatus::Unable {
        return Ok(StructuredCrawlResult {
            status: ExtractStatus::Unable,
            language,
            title,
            summary: summary.clone(),
            items: vec![],
            sources: vec![],
            follow_up_urls: vec![],
            unable_reason: Some(summary),
        });
    }

    let items = value
        .get("items")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let label = item.get("label")?.as_str()?.to_string();
                    let value = item.get("value")?.as_str()?.to_string();
                    let evidence = item
                        .get("evidence")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    Some(CrawlItem {
                        label,
                        value,
                        evidence,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let sources = value
        .get("sources")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    let url = s.get("url")?.as_str()?.to_string();
                    let note = s
                        .get("note")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(CrawlSourceRef { url, note })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let follow_up_urls = value
        .get("follow_up_urls")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let unable_reason = value
        .get("unable_reason")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(StructuredCrawlResult {
        status,
        language,
        title,
        summary,
        items,
        sources,
        follow_up_urls,
        unable_reason,
    })
}

fn extract_json_blob(s: &str) -> DomainResult<&str> {
    if let Some(start) = s.find("```") {
        let after = &s[start + 3..];
        let after = after
            .strip_prefix("json")
            .or_else(|| after.strip_prefix("JSON"))
            .unwrap_or(after);
        let after = after.trim_start_matches(['\n', '\r', ' ']);
        if let Some(end) = after.find("```") {
            return Ok(after[..end].trim());
        }
    }

    if let (Some(start), Some(end)) = (s.find('{'), s.rfind('}')) {
        if end > start {
            return Ok(&s[start..=end]);
        }
    }

    Err(DomainError::Parse(
        "no JSON object found in LLM response".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean_json() {
        let raw = r#"{"status":"ok","language":"en","title":"T","summary":"S","items":[{"label":"a","value":"b"}],"sources":[],"follow_up_urls":[]}"#;
        let r = parse_structured_result(raw).unwrap();
        assert_eq!(r.status, ExtractStatus::Ok);
        assert_eq!(r.items.len(), 1);
    }

    #[test]
    fn parses_fenced_json() {
        let raw = "Here you go:\n```json\n{\"status\":\"partial\",\"language\":\"fa\",\"title\":\"\",\"summary\":\"خلاصه\",\"items\":[],\"sources\":[],\"follow_up_urls\":[]}\n```";
        let r = parse_structured_result(raw).unwrap();
        assert_eq!(r.status, ExtractStatus::Partial);
        assert_eq!(r.language, "fa");
    }

    #[test]
    fn promotes_refusal_summary() {
        let raw = r#"{"status":"ok","language":"en","title":"","summary":"I couldn't access the page.","items":[],"sources":[],"follow_up_urls":[]}"#;
        let r = parse_structured_result(raw).unwrap();
        assert_eq!(r.status, ExtractStatus::Unable);
    }
}
