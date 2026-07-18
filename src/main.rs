use smart_hawk::bootstrap::{init_tracing, Config};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    init_tracing();

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "failed to load configuration");
            std::process::exit(1);
        }
    };

    info!(
        db = %config.database_url,
        workers = config.worker_pool_size,
        "smart hawk waking up"
    );

    if let Err(e) = smart_hawk::bootstrap::run(config).await {
        error!(error = %e, "fatal error");
        std::process::exit(1);
    }
}
