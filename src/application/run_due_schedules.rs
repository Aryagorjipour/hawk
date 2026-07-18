use std::sync::Arc;

use tracing::{error, info};

use crate::domain::{CrawlSource, DomainEvent, DomainResult};
use crate::infrastructure::EventBus;
use crate::ports::EmailMessage;
use crate::ports::{Clock, Mailer, ScheduleRepository, UserRepository};

use super::start_crawl::{EnqueueCrawl, StartCrawlService};

pub struct RunDueSchedulesService {
    pub users: Arc<dyn UserRepository>,
    pub schedules: Arc<dyn ScheduleRepository>,
    pub crawls: Arc<StartCrawlService>,
    pub mailer: Arc<dyn Mailer>,
    pub clock: Arc<dyn Clock>,
    pub events: EventBus,
    /// Callback to deliver chat messages: (telegram_user_id, text)
    pub chat_notify:
        Arc<dyn Fn(i64, String) -> futures::future::BoxFuture<'static, ()> + Send + Sync>,
}

impl RunDueSchedulesService {
    pub async fn tick(&self, limit: i64) -> DomainResult<usize> {
        let now = self.clock.now();
        let due = self.schedules.due(now, limit).await?;
        let mut ran = 0usize;
        for mut schedule in due {
            let user = match self.users.get_by_id(schedule.user_id).await? {
                Some(u) => u,
                None => continue,
            };

            if schedule.delivery.send_trigger_message {
                let label = schedule
                    .label
                    .clone()
                    .unwrap_or_else(|| schedule.start_url.clone());
                (self.chat_notify)(
                    user.telegram_user_id.get(),
                    format!("⏱ Schedule firing: {label}"),
                )
                .await;
            }

            let job = match self
                .crawls
                .enqueue(EnqueueCrawl {
                    user_id: user.id,
                    url: schedule.start_url.clone(),
                    prompt: schedule.user_prompt.clone(),
                    source: CrawlSource::Schedule,
                    schedule_id: Some(schedule.id),
                })
                .await
            {
                Ok(j) => j,
                Err(e) => {
                    error!(error = %e, schedule_id = %schedule.id, "schedule_enqueue_failed");
                    // Still advance next_run to avoid tight loop on permanent errors
                    let _ = schedule.bump_after_run(
                        crate::domain::CrawlJobId::new(),
                        &user.timezone,
                        self.clock.now(),
                    );
                    let _ = self.schedules.update(&schedule).await;
                    continue;
                }
            };

            let finished = self.crawls.execute_job(job).await?;

            if schedule.delivery.send_chat {
                let text = finished
                    .result_pretty
                    .clone()
                    .or_else(|| finished.error_detail.clone())
                    .unwrap_or_else(|| "Schedule crawl finished.".into());
                (self.chat_notify)(user.telegram_user_id.get(), text).await;
            }

            if schedule.delivery.send_email {
                if let Some(email) = &user.email {
                    if self.mailer.is_configured() {
                        let subject = format!(
                            "[Smart Hawk] {}",
                            finished
                                .result
                                .as_ref()
                                .map(|r| r.title.as_str())
                                .filter(|t| !t.is_empty())
                                .unwrap_or(schedule.start_url.as_str())
                        );
                        let body = finished
                            .result_pretty
                            .clone()
                            .or_else(|| finished.error_detail.clone())
                            .unwrap_or_default();
                        if let Err(e) = self
                            .mailer
                            .send(EmailMessage {
                                to: email.clone(),
                                subject,
                                text_body: body,
                                html_body: None,
                            })
                            .await
                        {
                            error!(error = %e, "schedule_email_failed");
                        }
                    } else {
                        (self.chat_notify)(
                            user.telegram_user_id.get(),
                            "Email delivery requested but SMTP is not configured on the server."
                                .into(),
                        )
                        .await;
                    }
                }
            }

            schedule.bump_after_run(finished.id, &user.timezone, self.clock.now())?;
            self.schedules.update(&schedule).await?;
            self.events.publish(DomainEvent::ScheduleFired {
                user_id: user.id,
                schedule_id: schedule.id,
                crawl_id: finished.id,
                at: self.clock.now(),
            });
            ran += 1;
            info!(schedule_id = %schedule.id, "schedule_run_complete");
        }
        Ok(ran)
    }
}
