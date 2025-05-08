use axum::{
    Router,
    routing::{post, put},
};
use mempool::{
    app_state::AppState,
    error::AppError,
    handlers::{handle_drain, handle_txn_submit},
    mempool::mempool::MemPool,
};
use std::error::Error;
use tokio::signal;
use tracing::info;

pub async fn run_full_server<M: MemPool + Default + Clone + 'static>(
    port: u16,
) -> Result<(), Box<dyn Error>> {
    let app_state = AppState {
        mempool: M::default(),
    };

    let app = Router::new()
        .route("/submit", post(handle_txn_submit::<M>))
        .route("/drain", put(handle_drain::<M>))
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
