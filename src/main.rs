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
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting up");
    let app_state = AppState {
        mempool: ActiveMemPool::default(),
    };

    let app = Router::new()
        .route("/submit", post(handle_txn_submit::<ActiveMemPool>))
        .route("/drain", put(handle_drain::<ActiveMemPool>))
        .with_state(app_state);

    info!("Listening on 8000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .map_err(|e| AppError::AxumServe(e.to_string()))?;
    axum::serve(listener, app)
        .await
        .map_err(|e| AppError::AxumServe(e.to_string()))?;

    Ok(())
}
