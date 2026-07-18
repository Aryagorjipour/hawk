PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    telegram_user_id INTEGER NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    email TEXT,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    locale TEXT NOT NULL DEFAULT 'en',
    onboarding_status TEXT NOT NULL,
    provider TEXT,
    base_url TEXT,
    api_key_ciphertext BLOB,
    api_key_nonce BLOB,
    model_id TEXT,
    connection_verified_at TEXT,
    bonus_crawl_credits INTEGER NOT NULL DEFAULT 0,
    bonus_schedule_slots INTEGER NOT NULL DEFAULT 0,
    free_crawls_used_today INTEGER NOT NULL DEFAULT 0,
    free_crawls_day TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS conversation_states (
    telegram_user_id INTEGER PRIMARY KEY NOT NULL,
    state_kind TEXT NOT NULL,
    state_payload TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS crawl_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source TEXT NOT NULL,
    schedule_id TEXT,
    start_url TEXT NOT NULL,
    user_prompt TEXT NOT NULL,
    status TEXT NOT NULL,
    pages_fetched INTEGER NOT NULL DEFAULT 0,
    result_json TEXT,
    result_pretty TEXT,
    error_kind TEXT,
    error_detail TEXT,
    created_at TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_crawl_jobs_user_created ON crawl_jobs(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_crawl_jobs_status ON crawl_jobs(status);

CREATE TABLE IF NOT EXISTS schedules (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label TEXT,
    start_url TEXT NOT NULL,
    user_prompt TEXT NOT NULL,
    recurrence_json TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,
    send_chat INTEGER NOT NULL DEFAULT 1,
    send_email INTEGER NOT NULL DEFAULT 0,
    send_trigger_msg INTEGER NOT NULL DEFAULT 1,
    next_run_at TEXT NOT NULL,
    last_run_at TEXT,
    last_crawl_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_schedules_due ON schedules(active, next_run_at);

CREATE TABLE IF NOT EXISTS history_entries (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    crawl_job_id TEXT NOT NULL REFERENCES crawl_jobs(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    start_url TEXT NOT NULL,
    prompt_snippet TEXT NOT NULL,
    result_pretty TEXT,
    error_detail TEXT,
    source TEXT NOT NULL,
    occurred_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_history_user_time ON history_entries(user_id, occurred_at DESC);

CREATE TABLE IF NOT EXISTS stars_payments (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    telegram_payment_charge_id TEXT NOT NULL UNIQUE,
    pack_id TEXT NOT NULL,
    stars_amount INTEGER NOT NULL,
    credits_granted INTEGER NOT NULL,
    slots_granted INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS crawl_page_traces (
    id TEXT PRIMARY KEY NOT NULL,
    crawl_job_id TEXT NOT NULL REFERENCES crawl_jobs(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    fetch_mode TEXT NOT NULL,
    http_status INTEGER,
    ok INTEGER NOT NULL,
    error_detail TEXT,
    fetched_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_page_traces_job ON crawl_page_traces(crawl_job_id);
