use scraper::{Html, Selector};

use crate::domain::MAX_PAGE_CONTEXT_CHARS;

pub struct ExtractedContent {
    pub title: Option<String>,
    pub text: String,
    pub raw_excerpt: String,
}

pub fn extract_from_html(html: &str, content_type: Option<&str>) -> ExtractedContent {
    let ct = content_type.unwrap_or("").to_ascii_lowercase();
    if ct.contains("application/json") || looks_like_json(html) {
        return extract_json(html);
    }

    let document = Html::parse_document(html);
    let title = document
        .select(&Selector::parse("title").unwrap())
        .next()
        .map(|n| collapse_ws(&n.text().collect::<String>()))
        .filter(|s| !s.is_empty());

    // Drop script/style noise via html2text on a cleaned-ish body
    let body_html = document
        .select(&Selector::parse("body").unwrap())
        .next()
        .map(|b| b.html())
        .unwrap_or_else(|| html.to_string());

    let cleaned = strip_noise_tags(&body_html);
    let mut text =
        html2text::from_read(cleaned.as_bytes(), 100).unwrap_or_else(|_| collapse_ws(&body_html));
    text = collapse_ws_multiline(&text);
    if text.chars().count() > MAX_PAGE_CONTEXT_CHARS {
        text = text.chars().take(MAX_PAGE_CONTEXT_CHARS).collect();
        text.push_str("\n…[truncated]");
    }

    let raw_excerpt: String = html.chars().take(500).collect();

    ExtractedContent {
        title,
        text,
        raw_excerpt,
    }
}

fn extract_json(raw: &str) -> ExtractedContent {
    let pretty = match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| raw.to_string()),
        Err(_) => raw.to_string(),
    };
    let mut text = pretty;
    if text.chars().count() > MAX_PAGE_CONTEXT_CHARS {
        text = text.chars().take(MAX_PAGE_CONTEXT_CHARS).collect();
        text.push_str("\n…[truncated]");
    }
    ExtractedContent {
        title: Some("JSON document".into()),
        text,
        raw_excerpt: raw.chars().take(500).collect(),
    }
}

fn looks_like_json(s: &str) -> bool {
    let t = s.trim_start();
    t.starts_with('{') || t.starts_with('[')
}

fn strip_noise_tags(html: &str) -> String {
    // Lightweight scrub: remove script/style blocks
    let re_script = regex::Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("script regex");
    let re_style = regex::Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("style regex");
    let re_noscript =
        regex::Regex::new(r"(?is)<noscript[^>]*>.*?</noscript>").expect("noscript regex");
    let s = re_script.replace_all(html, " ");
    let s = re_style.replace_all(&s, " ");
    re_noscript.replace_all(&s, " ").into_owned()
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collapse_ws_multiline(s: &str) -> String {
    let mut out = String::new();
    let mut blank = 0;
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() {
            blank += 1;
            if blank <= 1 {
                out.push('\n');
            }
        } else {
            blank = 0;
            out.push_str(t);
            out.push('\n');
        }
    }
    out
}
