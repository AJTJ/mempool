use axum::{
    Router,
    routing::{post, put},
};
use mempool::{
    app_state::AppState,
    error::AppError,
    handlers::{handle_commit, handle_drain, handle_release, handle_reserve, handle_txn_submit},
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

    let app = router(app_state);

    info!("Listening on 8000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .map_err(|e| AppError::AxumServe(e.to_string()))?;
    axum::serve(listener, app)
        .await
        .map_err(|e| AppError::AxumServe(e.to_string()))?;

    Ok(())
}

pub fn router(state: AppState<ActiveMemPool>) -> Router {
    let core_routes = Router::new()
        .route("/submit", post(handle_txn_submit::<ActiveMemPool>))
        .route("/drain", put(handle_drain::<ActiveMemPool>));

    #[cfg(feature = "mempool-skiplist")]
    let core_routes = core_routes
        .route("/reserve", post(handle_reserve))
        .route("/commit", post(handle_commit))
        .route("/release", post(handle_release));

    core_routes.with_state(state)
}
