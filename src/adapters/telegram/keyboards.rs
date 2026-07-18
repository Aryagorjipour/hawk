use teloxide::types::{
    InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup, ReplyMarkup,
};

use crate::adapters::i18n::I18n;
use crate::application::MODELS_PER_PAGE;
use crate::domain::{Locale, ProviderKind, Schedule, ALL_PACKS};

pub fn main_menu(i18n: &I18n, locale: Locale) -> ReplyMarkup {
    ReplyMarkup::Keyboard(
        KeyboardMarkup::new(vec![
            vec![
                KeyboardButton::new(i18n.t0(locale, "btn-crawl")),
                KeyboardButton::new(i18n.t0(locale, "btn-schedule")),
            ],
            vec![
                KeyboardButton::new(i18n.t0(locale, "btn-settings")),
                KeyboardButton::new(i18n.t0(locale, "btn-history")),
            ],
            vec![
                KeyboardButton::new(i18n.t0(locale, "btn-usage")),
                KeyboardButton::new(i18n.t0(locale, "btn-about")),
            ],
        ])
        .resize_keyboard()
        .persistent(),
    )
}

pub fn provider_keyboard(i18n: &I18n, locale: Locale, prefix: &str) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    for (i, p) in ProviderKind::all().iter().enumerate() {
        let key = format!("provider-{}", p.as_str());
        row.push(InlineKeyboardButton::callback(
            i18n.t0(locale, &key),
            format!("{prefix}:provider:{}", p.as_str()),
        ));
        if row.len() == 2 || i + 1 == ProviderKind::all().len() {
            rows.push(std::mem::take(&mut row));
        }
    }
    InlineKeyboardMarkup::new(rows)
}

pub fn default_name_keyboard(i18n: &I18n, locale: Locale, name: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        i18n.t(locale, "btn-use-default", &[("name", name.to_string())]),
        format!("onboard:default_name:{name}"),
    )]])
}

pub fn models_keyboard(
    models: &[String],
    page: usize,
    pages: usize,
    prefix: &str,
) -> InlineKeyboardMarkup {
    let start = page * MODELS_PER_PAGE;
    let end = (start + MODELS_PER_PAGE).min(models.len());
    let mut rows: Vec<Vec<InlineKeyboardButton>> = models[start..end]
        .iter()
        .map(|m| {
            vec![InlineKeyboardButton::callback(
                m.clone(),
                format!("{prefix}:model:{m}"),
            )]
        })
        .collect();

    let mut nav = Vec::new();
    if page > 0 {
        nav.push(InlineKeyboardButton::callback(
            "« Prev",
            format!("{prefix}:page:{}", page - 1),
        ));
    }
    if page + 1 < pages {
        nav.push(InlineKeyboardButton::callback(
            "Next »",
            format!("{prefix}:page:{}", page + 1),
        ));
    }
    if !nav.is_empty() {
        rows.push(nav);
    }
    InlineKeyboardMarkup::new(rows)
}

pub fn settings_keyboard(i18n: &I18n, locale: Locale) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            i18n.t0(locale, "settings-name"),
            "settings:name",
        )],
        vec![InlineKeyboardButton::callback(
            i18n.t0(locale, "settings-email"),
            "settings:email",
        )],
        vec![InlineKeyboardButton::callback(
            i18n.t0(locale, "settings-timezone"),
            "settings:tz",
        )],
        vec![InlineKeyboardButton::callback(
            i18n.t0(locale, "settings-ai"),
            "settings:ai",
        )],
        vec![
            InlineKeyboardButton::callback("English", "settings:lang:en"),
            InlineKeyboardButton::callback("فارسی", "settings:lang:fa"),
        ],
        vec![InlineKeyboardButton::callback(
            i18n.t0(locale, "settings-delete"),
            "settings:delete",
        )],
    ])
}

pub fn schedules_keyboard(
    i18n: &I18n,
    locale: Locale,
    schedules: &[Schedule],
    used: u32,
    max: u32,
) -> (String, InlineKeyboardMarkup) {
    let mut text = i18n.t(
        locale,
        "schedule-hub",
        &[("used", used.to_string()), ("max", max.to_string())],
    );
    text.push('\n');
    if schedules.is_empty() {
        text.push_str(&i18n.t0(locale, "schedule-empty"));
    } else {
        for (i, s) in schedules.iter().enumerate() {
            let flag = if s.active { "✅" } else { "⏸" };
            text.push_str(&format!(
                "\n{}. {} {} — {}",
                i + 1,
                flag,
                host_of(&s.start_url),
                truncate(&s.user_prompt, 40)
            ));
        }
    }
    let mut rows: Vec<Vec<InlineKeyboardButton>> = schedules
        .iter()
        .map(|s| {
            vec![InlineKeyboardButton::callback(
                format!("Open {}", host_of(&s.start_url)),
                format!("sched:open:{}", s.id),
            )]
        })
        .collect();
    rows.push(vec![InlineKeyboardButton::callback(
        i18n.t0(locale, "schedule-new"),
        "sched:new",
    )]);
    (text, InlineKeyboardMarkup::new(rows))
}

pub fn schedule_detail_keyboard(
    i18n: &I18n,
    locale: Locale,
    id: &str,
    active: bool,
) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            i18n.t0(locale, "schedule-trigger-now"),
            format!("sched:run:{id}"),
        )],
        vec![InlineKeyboardButton::callback(
            if active {
                i18n.t0(locale, "schedule-deactivate")
            } else {
                i18n.t0(locale, "schedule-activate")
            },
            format!("sched:toggle:{id}"),
        )],
        vec![InlineKeyboardButton::callback(
            i18n.t0(locale, "btn-delete"),
            format!("sched:del:{id}"),
        )],
        vec![InlineKeyboardButton::callback(
            i18n.t0(locale, "btn-history"),
            format!("sched:hist:{id}"),
        )],
        vec![InlineKeyboardButton::callback(
            i18n.t0(locale, "btn-back"),
            "sched:list",
        )],
    ])
}

pub fn usage_packs_hint_keyboard(i18n: &I18n, locale: Locale) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        i18n.t0(locale, "about-buy"),
        "usage:packs",
    )]])
}

pub fn recurrence_keyboard(i18n: &I18n, locale: Locale) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            i18n.t0(locale, "schedule-interval"),
            "sched:rec:interval",
        )],
        vec![InlineKeyboardButton::callback(
            i18n.t0(locale, "schedule-daily"),
            "sched:rec:daily",
        )],
        vec![InlineKeyboardButton::callback(
            i18n.t0(locale, "schedule-weekly"),
            "sched:rec:weekly",
        )],
    ])
}

pub fn interval_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("15m", "sched:int:15"),
            InlineKeyboardButton::callback("1h", "sched:int:60"),
            InlineKeyboardButton::callback("6h", "sched:int:360"),
        ],
        vec![
            InlineKeyboardButton::callback("12h", "sched:int:720"),
            InlineKeyboardButton::callback("24h", "sched:int:1440"),
        ],
    ])
}

pub fn delivery_keyboard(chat: bool, email: bool, trigger: bool) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            format!("Chat {}", on_off(chat)),
            "sched:deliv:chat",
        )],
        vec![InlineKeyboardButton::callback(
            format!("Email {}", on_off(email)),
            "sched:deliv:email",
        )],
        vec![InlineKeyboardButton::callback(
            format!("Trigger msg {}", on_off(trigger)),
            "sched:deliv:trigger",
        )],
        vec![InlineKeyboardButton::callback(
            "Save schedule",
            "sched:deliv:save",
        )],
    ])
}

pub fn about_packs_keyboard() -> InlineKeyboardMarkup {
    let mut rows = Vec::new();
    for p in ALL_PACKS {
        rows.push(vec![InlineKeyboardButton::callback(
            format!(
                "{}★ → {} crawls +{} slots",
                p.stars, p.credits, p.schedule_slots
            ),
            format!("pay:pack:{}", p.id),
        )]);
    }
    InlineKeyboardMarkup::new(rows)
}

pub fn history_keyboard(entries: &[(String, String)]) -> InlineKeyboardMarkup {
    // (id, label)
    let rows = entries
        .iter()
        .map(|(id, label)| {
            vec![InlineKeyboardButton::callback(
                label.clone(),
                format!("hist:view:{id}"),
            )]
        })
        .collect::<Vec<_>>();
    InlineKeyboardMarkup::new(rows)
}

pub fn timezone_keyboard() -> InlineKeyboardMarkup {
    let zones = [
        "UTC",
        "Europe/London",
        "Europe/Berlin",
        "Europe/Istanbul",
        "Asia/Tehran",
        "Asia/Dubai",
        "Asia/Tokyo",
        "America/New_York",
        "America/Los_Angeles",
    ];
    let rows = zones
        .chunks(2)
        .map(|chunk| {
            chunk
                .iter()
                .map(|z| {
                    InlineKeyboardButton::callback((*z).to_string(), format!("settings:setz:{z}"))
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    InlineKeyboardMarkup::new(rows)
}

fn on_off(v: bool) -> &'static str {
    if v {
        "✅"
    } else {
        "❌"
    }
}

fn host_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
        .unwrap_or_else(|| truncate(url, 24))
}

fn truncate(s: &str, max: usize) -> String {
    let mut out: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        out.push('…');
    }
    out
}
