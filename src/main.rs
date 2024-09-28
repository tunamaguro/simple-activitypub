use std::time::Duration;

use axum::{routing::get, Router};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{normalize_path::NormalizePathLayer, timeout::TimeoutLayer, trace::TraceLayer};

#[tokio::main]
async fn main() {
    init_tracing();

    let app = Router::new()
        .route("/", get(|| async { "Hello World!" }))
        .layer(
            ServiceBuilder::new()
                .layer(NormalizePathLayer::trim_trailing_slash())
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::new(Duration::from_secs(10))),
        );

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("server listning on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let level = if cfg!(debug_assertions) {
            "trace"
        } else {
            "debug"
        };
        format!(
            "{}={},tower_http=debug,axum::rejection=trace",
            env!("CARGO_CRATE_NAME"),
            level
        )
        .into()
    });

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true);

    #[cfg(debug_assertions)]
    let fmt_layer = fmt_layer.with_ansi(true).pretty();
    #[cfg(not(debug_assertions))]
    let fmt_layer = fmt_layer.json();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}
