use smart_hawk::bootstrap::{init_tracing, Config};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Failures before tracing still go to stderr (helps Docker “no logs” debugging)
    eprintln!("smart-hawk: starting…");

    init_tracing();

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("smart-hawk: config error: {e}");
            error!(error = %e, "failed to load configuration");
            std::process::exit(1);
        }
    };

    info!(
        db = %config.database_url,
        workers = config.worker_pool_size,
        email = %config.email_diag(),
        "smart hawk waking up"
    );

    if let Err(e) = smart_hawk::bootstrap::run(config).await {
        eprintln!("smart-hawk: fatal: {e}");
        error!(error = %e, "fatal error");
        std::process::exit(1);
    }
}
