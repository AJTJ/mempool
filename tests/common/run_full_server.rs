use axum::{
    Router,
    routing::{post, put},
};
use mempool::{
    app_state::AppState,
    error::AppError,
    handlers::{handle_drain, handle_txn_submit},
    mempool::ActiveMemPool,
};
use std::error::Error;
use tokio::signal;
use tracing::info;

pub async fn run_full_server(port: u16) -> Result<(), Box<dyn Error>> {
    let app_state = AppState {
        mempool: ActiveMemPool::default(),
    };

    let app = Router::new()
        .route("/submit", post(handle_txn_submit::<ActiveMemPool>))
        .route("/drain", put(handle_drain::<ActiveMemPool>))
        .with_state(app_state);

    info!("Listening on {}", port);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .map_err(|e| AppError::AxumServe(e.to_string()))?;

    let shutdown = signal::ctrl_c();

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            shutdown
                .await
                .expect("Failed to listen for shutdown signal");
            info!("Shutting down gracefully...");
        })
        .await
        .map_err(|e| AppError::AxumServe(e.to_string()))?;

    Ok(())
}
