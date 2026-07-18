use std::collections::VecDeque;
use std::sync::Arc;

use tracing::{info, warn};
use url::Url;

use crate::adapters::llm::build_llm_client;
use crate::domain::{
    looks_like_refusal, parse_crawl_url, prettify_result, CrawlJob, CrawlSource, DomainError,
    DomainEvent, DomainResult, ExtractStatus, HistoryEntry, PageBudget, StructuredCrawlResult,
    User, UserId, HISTORY_CAP_PER_USER,
};
use crate::infrastructure::EventBus;
use crate::ports::{
    Clock, CrawlRepository, ExtractRequest, HistoryRepository, PageContext, PageFetcher,
    PageTraceRepository, SecretBox, UserRepository,
};

pub struct StartCrawlService {
    pub users: Arc<dyn UserRepository>,
    pub crawls: Arc<dyn CrawlRepository>,
    pub history: Arc<dyn HistoryRepository>,
    pub traces: Arc<dyn PageTraceRepository>,
    pub fetcher: Arc<dyn PageFetcher>,
    pub secrets: Arc<dyn SecretBox>,
    pub clock: Arc<dyn Clock>,
    pub events: EventBus,
}

pub struct EnqueueCrawl {
    pub user_id: UserId,
    pub url: String,
    pub prompt: String,
    pub source: CrawlSource,
    pub schedule_id: Option<crate::domain::ScheduleId>,
}

impl StartCrawlService {
    pub async fn validate_url(&self, raw: &str) -> DomainResult<Url> {
        parse_crawl_url(raw)
    }

    pub async fn enqueue(&self, cmd: EnqueueCrawl) -> DomainResult<CrawlJob> {
        let mut user = self
            .users
            .get_by_id(cmd.user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        user.ensure_ready_to_crawl()?;

        if self.crawls.has_active_for_user(user.id).await? {
            return Err(DomainError::CrawlAlreadyRunning);
        }

        let url = parse_crawl_url(&cmd.url)?;
        let now = self.clock.now();
        user.credits.try_consume_crawl(now)?;
        self.users.update(&user).await?;

        let job = CrawlJob::new(
            user.id,
            cmd.source,
            cmd.schedule_id,
            url.to_string(),
            cmd.prompt,
            now,
        )?;
        self.crawls.insert(&job).await?;
        info!(crawl_id = %job.id, user_id = %user.id, "crawl_enqueued");
        Ok(job)
    }

    pub async fn execute_job(&self, mut job: CrawlJob) -> DomainResult<CrawlJob> {
        let now = self.clock.now();
        if job.status == crate::domain::CrawlStatus::Queued {
            job.mark_running(now)?;
            self.crawls.update(&job).await?;
        }

        let user = self
            .users
            .get_by_id(job.user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        let ai = user.ai_config.as_ref().ok_or(DomainError::AiNotVerified)?;
        let llm = build_llm_client(ai, self.secrets.as_ref())?;

        let mut budget = PageBudget::default_crawl();
        let mut queue = VecDeque::new();
        queue.push_back(job.start_url.clone());
        let mut collected: Vec<PageContext> = Vec::new();
        let mut last_result: Option<StructuredCrawlResult> = None;
        let mut first_page = true;

        while let Some(raw_url) = queue.pop_front() {
            if !budget.try_consume() {
                break;
            }
            let url = match parse_crawl_url(&raw_url) {
                Ok(u) => u,
                Err(e) => {
                    warn!(error = %e, url = %raw_url, "skip_invalid_followup");
                    continue;
                }
            };

            let page = match self.fetcher.fetch_resilient(&url).await {
                Ok(p) => p,
                Err(e) => {
                    let _ = self
                        .traces
                        .insert(
                            job.id,
                            url.as_str(),
                            "http",
                            None,
                            false,
                            Some(&e.to_string()),
                            self.clock.now(),
                        )
                        .await;
                    if first_page {
                        let now = self.clock.now();
                        job.mark_failed(e.error_code(), e.user_message(), budget.used(), now)?;
                        self.crawls.update(&job).await?;
                        self.record_history(&job).await?;
                        self.events.publish(DomainEvent::CrawlFailed {
                            user_id: job.user_id,
                            crawl_id: job.id,
                            kind: "fetch".into(),
                            at: now,
                        });
                        return Ok(job);
                    }
                    continue;
                }
            };

            let _ = self
                .traces
                .insert(
                    job.id,
                    &page.final_url,
                    page.fetch_mode.as_str(),
                    Some(page.status as i32),
                    page.is_usable(),
                    None,
                    self.clock.now(),
                )
                .await;

            if first_page {
                first_page = false;
                let sample = page.fingerprint_sample();
                if !sample.is_empty() {
                    match llm
                        .sanity_check_title(&ai.model_id, &sample, &page.text)
                        .await
                    {
                        Ok(reply) => {
                            let ok = reply.to_ascii_lowercase().contains(
                                &sample
                                    .chars()
                                    .take(20)
                                    .collect::<String>()
                                    .to_ascii_lowercase(),
                            ) || sample.to_ascii_lowercase().contains(
                                &reply
                                    .chars()
                                    .take(20)
                                    .collect::<String>()
                                    .to_ascii_lowercase(),
                            );
                            if !ok {
                                info!(
                                    crawl_id = %job.id,
                                    "sanity_check_loose_mismatch_continuing_with_engine_content"
                                );
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "sanity_check_failed_continuing");
                        }
                    }
                }
            }

            collected.push(PageContext {
                url: page.final_url.clone(),
                title: page.title.clone(),
                text: page.text.clone(),
                fetch_mode: page.fetch_mode.as_str().to_string(),
            });

            let extract_req = ExtractRequest {
                model: ai.model_id.clone(),
                user_prompt: job.user_prompt.clone(),
                pages: collected.clone(),
                response_language_hint: Some(user.locale.as_str().to_string()),
            };

            match llm.extract(extract_req).await {
                Ok(result) => {
                    if matches!(result.status, ExtractStatus::Unable)
                        || looks_like_refusal(&result.summary)
                    {
                        last_result = Some(result);
                        break;
                    }
                    for fu in &result.follow_up_urls {
                        if budget.remaining() == 0 {
                            break;
                        }
                        if parse_crawl_url(fu).is_ok()
                            && !queue.iter().any(|u| u == fu)
                            && !collected.iter().any(|p| p.url == *fu)
                        {
                            queue.push_back(fu.clone());
                        }
                    }
                    last_result = Some(result);
                    if queue.is_empty() {
                        break;
                    }
                }
                Err(e) => {
                    let now = self.clock.now();
                    job.mark_failed(e.error_code(), e.user_message(), budget.used(), now)?;
                    self.crawls.update(&job).await?;
                    self.record_history(&job).await?;
                    self.events.publish(DomainEvent::CrawlFailed {
                        user_id: job.user_id,
                        crawl_id: job.id,
                        kind: "llm".into(),
                        at: now,
                    });
                    return Ok(job);
                }
            }
        }

        let now = self.clock.now();
        match last_result {
            Some(result) if result.is_usable() => {
                let pretty = prettify_result(&result);
                job.mark_succeeded(result, pretty, budget.used(), now)?;
                self.crawls.update(&job).await?;
                self.record_history(&job).await?;
                self.events.publish(DomainEvent::CrawlCompleted {
                    user_id: job.user_id,
                    crawl_id: job.id,
                    at: now,
                });
            }
            Some(result) => {
                let reason = result
                    .unable_reason
                    .clone()
                    .unwrap_or_else(|| result.summary.clone());
                job.mark_failed("unable", reason, budget.used(), now)?;
                self.crawls.update(&job).await?;
                self.record_history(&job).await?;
                self.events.publish(DomainEvent::CrawlFailed {
                    user_id: job.user_id,
                    crawl_id: job.id,
                    kind: "unable".into(),
                    at: now,
                });
            }
            None => {
                job.mark_failed(
                    "empty",
                    "no usable pages or extraction result".to_string(),
                    budget.used(),
                    now,
                )?;
                self.crawls.update(&job).await?;
                self.record_history(&job).await?;
                self.events.publish(DomainEvent::CrawlFailed {
                    user_id: job.user_id,
                    crawl_id: job.id,
                    kind: "empty".into(),
                    at: now,
                });
            }
        }

        Ok(job)
    }

    async fn record_history(&self, job: &CrawlJob) -> DomainResult<()> {
        let entry = HistoryEntry::from_finished_job(job, self.clock.now());
        self.history.insert(&entry).await?;
        self.history
            .trim_to_cap(job.user_id, HISTORY_CAP_PER_USER)
            .await?;
        Ok(())
    }

    pub async fn reload_user(&self, id: UserId) -> DomainResult<User> {
        self.users
            .get_by_id(id)
            .await?
            .ok_or(DomainError::UserNotFound)
    }
}
