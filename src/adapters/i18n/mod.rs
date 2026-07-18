use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::domain::Locale;

static ARG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{\s*\$([a-zA-Z0-9_]+)\s*\}").unwrap());

#[derive(Clone)]
pub struct I18n {
    catalogs: Arc<HashMap<Locale, HashMap<String, String>>>,
}

impl I18n {
    pub fn load_embedded() -> Self {
        let mut catalogs = HashMap::new();
        catalogs.insert(
            Locale::En,
            parse_ftl(include_str!("../../../locales/en/main.ftl")),
        );
        catalogs.insert(
            Locale::Fa,
            parse_ftl(include_str!("../../../locales/fa/main.ftl")),
        );
        Self {
            catalogs: Arc::new(catalogs),
        }
    }

    pub fn t(&self, locale: Locale, key: &str, args: &[(&str, String)]) -> String {
        let catalog = self
            .catalogs
            .get(&locale)
            .or_else(|| self.catalogs.get(&Locale::En))
            .expect("en catalog");
        let template = catalog
            .get(key)
            .cloned()
            .or_else(|| {
                self.catalogs
                    .get(&Locale::En)
                    .and_then(|c| c.get(key).cloned())
            })
            .unwrap_or_else(|| key.to_string());

        let mut map: HashMap<&str, &str> = HashMap::new();
        for (k, v) in args {
            map.insert(*k, v.as_str());
        }

        ARG_RE
            .replace_all(&template, |caps: &regex::Captures| {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                map.get(name).copied().unwrap_or("").to_string()
            })
            .into_owned()
    }

    pub fn t0(&self, locale: Locale, key: &str) -> String {
        self.t(locale, key, &[])
    }
}

/// Minimal FTL-like parser: `key = value` with multiline continuation indented.
fn parse_ftl(src: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_val = String::new();

    for line in src.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if !k.starts_with(' ') && !k.starts_with('\t') {
                if let Some(prev) = current_key.take() {
                    map.insert(prev, current_val.trim().to_string());
                    current_val.clear();
                }
                current_key = Some(k.trim().to_string());
                current_val = v.trim_start().to_string();
                continue;
            }
        }
        if current_key.is_some() {
            if !current_val.is_empty() {
                current_val.push('\n');
            }
            current_val.push_str(line.trim());
        }
    }
    if let Some(prev) = current_key {
        map.insert(prev, current_val.trim().to_string());
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_en_and_interpolates() {
        let i18n = I18n::load_embedded();
        let s = i18n.t(Locale::En, "welcome-back", &[("name", "Arya".into())]);
        assert!(s.contains("Arya"));
    }

    #[test]
    fn fa_fallback_keys_exist() {
        let i18n = I18n::load_embedded();
        let s = i18n.t0(Locale::Fa, "btn-crawl");
        assert!(!s.is_empty());
    }
}
