use std::time::Duration;

use axum::{
    http::{header, HeaderValue},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{normalize_path::NormalizePathLayer, timeout::TimeoutLayer, trace::TraceLayer};

const BASE_SERVER_URI: &str = "c0becd579a6677ee42ce067be08e544d.serveo.net";

#[tracing::instrument]
#[axum_macros::debug_handler]
async fn node_info() {}

#[tracing::instrument]
#[axum_macros::debug_handler]
async fn host_meta() -> impl IntoResponse {
    let xml = format!(
        r#"<?xml version="1.0"?>
<XRD xmlns="http://docs.oasis-open.org/ns/xri/xrd-1.0">
    <Link rel="lrdd" type="application/xrd+xml" template="https://{}/.well-known/webfinger?resource={{uri}}" />
</XRD>
"#,
        BASE_SERVER_URI
    );

    ([(header::CONTENT_TYPE, "application/xml")], xml)
}

#[tracing::instrument]
#[axum_macros::debug_handler]
async fn webfinger() -> impl IntoResponse {
    let res = serde_json::json!(
        {
            "subject": format!("acct:alice@{}",BASE_SERVER_URI),

            "links": [
                {
                    "rel": "self",
                    "type": "application/activity+json",
                    "href": format!("https://{}/actor",BASE_SERVER_URI)
                }
            ]
        }
    );

    Json(res)
}
// NOTE: Do not use these implementation at production!!
const PUBLIC_ACTOR_KEY: &str = include_str!("./actorKey/public.pem");
const PRIVATE_ACTOR_KEY: &str = include_str!("./actorKey/private.pem");
#[tracing::instrument]
#[axum_macros::debug_handler]
async fn person_handler() -> impl IntoResponse {
    let res = serde_json::json!(
        {
            "@context": [
                "https://www.w3.org/ns/activitystreams",
                "https://w3id.org/security/v1"
            ],

            "id": format!("https://{}/actor",BASE_SERVER_URI),
            "type": "Person",
            "preferredUsername": "alice",
            "inbox": format!("https://{}/inbox",BASE_SERVER_URI),

            "publicKey": {
                "id": format!("https://{}/actor#main-key",BASE_SERVER_URI),
                "owner":  format!("https://{}/actor",BASE_SERVER_URI),
                "publicKeyPem": PUBLIC_ACTOR_KEY
            }
        }
    );
    // Activity Stream document mime-type must be `application/activity+json`
    // See https://www.w3.org/TR/activitystreams-core/#h-syntaxconventions
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/activity+json"),
        )],
        Json(res),
    )
}

#[tokio::main]
async fn main() {
    init_tracing();

    let app = Router::new()
        .route("/host-meta", get(host_meta))
        .route("/.well-known/webfinger", get(webfinger))
        .route("/actor", get(person_handler))
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
