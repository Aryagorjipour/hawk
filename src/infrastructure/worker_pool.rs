use std::sync::Arc;

use tokio::sync::{mpsc, Semaphore};
use tracing::{error, info, Instrument};

pub struct WorkerPool<J: Send + 'static> {
    tx: mpsc::Sender<J>,
}

impl<J: Send + 'static> WorkerPool<J> {
    pub fn spawn<F, Fut>(pool_size: usize, queue_capacity: usize, handler: F) -> Self
    where
        F: Fn(J) -> Fut + Send + Sync + 'static + Clone,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<J>(queue_capacity);
        let sem = Arc::new(Semaphore::new(pool_size));

        tokio::spawn(async move {
            while let Some(job) = rx.recv().await {
                let permit = match sem.clone().acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => {
                        error!("worker semaphore closed");
                        break;
                    }
                };
                let handler = handler.clone();
                tokio::spawn(
                    async move {
                        handler(job).await;
                        drop(permit);
                    }
                    .instrument(tracing::info_span!("worker_job")),
                );
            }
            info!("worker pool receiver stopped");
        });

        Self { tx }
    }

    pub async fn submit(&self, job: J) -> Result<(), mpsc::error::SendError<J>> {
        self.tx.send(job).await
    }

    pub fn try_submit(&self, job: J) -> Result<(), mpsc::error::TrySendError<J>> {
        self.tx.try_send(job)
    }
}
