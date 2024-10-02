use std::time::SystemTime;

use axum::{
    http::{header, HeaderValue},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::Signer;
use openssl::{hash::MessageDigest, rsa::Padding};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{normalize_path::NormalizePathLayer, timeout::TimeoutLayer, trace::TraceLayer};

/// Your server domain
const BASE_SERVER_DOMAIN: &str = "855cc425539871507ed31abb8c428037.serveo.net";

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
        BASE_SERVER_DOMAIN
    );

    ([(header::CONTENT_TYPE, "application/xml")], xml)
}

#[tracing::instrument]
#[axum_macros::debug_handler]
async fn webfinger() -> impl IntoResponse {
    let res = serde_json::json!(
        {
            "subject": format!("acct:alice@{}",BASE_SERVER_DOMAIN),

            "links": [
                {
                    "rel": "self",
                    "type": "application/activity+json",
                    "href": format!("https://{}/actor",BASE_SERVER_DOMAIN)
                }
            ]
        }
    );

    Json(res)
}

const ACITIVITY_JSON: &str = "application/activity+json";

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

            "id": format!("https://{}/actor",BASE_SERVER_DOMAIN),
            "type": "Person",
            "preferredUsername": "alice",
            "inbox": format!("https://{}/inbox",BASE_SERVER_DOMAIN),

            "publicKey": {
                "id": format!("https://{}/actor#main-key",BASE_SERVER_DOMAIN),
                "owner":  format!("https://{}/actor",BASE_SERVER_DOMAIN),
                "publicKeyPem": PUBLIC_ACTOR_KEY
            }
        }
    );
    // Activity Stream document mime-type must be `application/activity+json`
    // See https://www.w3.org/TR/activitystreams-core/#h-syntaxconventions
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static(ACITIVITY_JSON),
        )],
        Json(res),
    )
}

const ACITIVITY_ACADEMY: &str = "activitypub.academy";
// Reply Note id
const REPLY_TO: &str = "https://activitypub.academy/@dabolia_ornorgliol/113233379073385013";

#[tracing::instrument]
#[axum_macros::debug_handler]
async fn post_note() {
    let document = serde_json::json!(
        {
            "@context": "https://www.w3.org/ns/activitystreams",

            "id": format!("https://{}/create-hello-world-my-server",BASE_SERVER_DOMAIN),
            "type": "Create",
            "actor": format!("https://{}/actor",BASE_SERVER_DOMAIN),

            "object": {
                "id": format!("https://{}/hello-world-my-server",BASE_SERVER_DOMAIN),
                "type": "Note",
                "published": "2018-06-23T17:17:11Z",
                "attributedTo": format!("https://{}/actor",BASE_SERVER_DOMAIN),
                "inReplyTo": REPLY_TO,
                "content": "<p>Hello world from my server</p>",
                "to": "https://www.w3.org/ns/activitystreams#Public"
            }
        }
    );
    let document = document.to_string();

    let rsa_private_key = Rsa::private_key_from_pem(PRIVATE_ACTOR_KEY.as_bytes()).unwrap();
    let pkey = PKey::from_rsa(rsa_private_key).unwrap();

    // Mastdon needs activity digest hash
    // https://docs.joinmastodon.org/spec/security/#digest
    let digest = openssl::hash::hash(MessageDigest::sha256(), document.as_bytes()).unwrap();
    let digest_64 = format!("SHA-256={}", openssl::base64::encode_block(digest.as_ref()));
    
    // https://docs.joinmastodon.org/spec/security/#http
    let date = httpdate::fmt_http_date(SystemTime::now());

    // HTTP Signature
    let mut signer = Signer::new(MessageDigest::sha256(), &pkey).unwrap();
    signer.set_rsa_padding(Padding::PKCS1).unwrap();
    let signed_string = format!(
        "(request-target): post /inbox\nhost: {}\ndate: {}\ndigest: {}",
        ACITIVITY_ACADEMY, date, digest_64
    );

    let signature = signer
        .sign_oneshot_to_vec(signed_string.as_bytes())
        .unwrap();
    let signature_64 = openssl::base64::encode_block(&signature);

    let header = format!(
        r#"keyId="https://{}/actor",algorithm="rsa-sha256",headers="(request-target) host date digest",signature="{}""#,
        BASE_SERVER_DOMAIN, signature_64
    );

    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(ACITIVITY_JSON),
    );
    headers.insert(header::ACCEPT, HeaderValue::from_static(ACITIVITY_JSON));
    headers.insert(header::HOST, HeaderValue::from_static(ACITIVITY_ACADEMY));
    headers.insert(header::DATE, HeaderValue::from_str(&date).unwrap());
    // HeaderName::from_static needs lowercase character
    // https://docs.rs/http/latest/http/header/struct.HeaderName.html#method.from_static
    headers.insert(
        header::HeaderName::from_static("signature"),
        HeaderValue::from_str(&header).unwrap(),
    );
    headers.insert(
        header::HeaderName::from_static("digest"),
        HeaderValue::from_str(&digest_64).unwrap(),
    );

    tracing::trace!(?headers);

    let res = reqwest::Client::new()
        .post("https://activitypub.academy/inbox")
        .headers(headers)
        .body(document)
        .send()
        .await;

    match res {
        Ok(res) => {
            let text = res.text().await;
            tracing::info!(?text)
        }
        Err(err) => {
            tracing::error!(?err)
        }
    };
}

#[tokio::main]
async fn main() {
    init_tracing();

    let app = Router::new()
        .route("/send-note", get(post_note))
        .route("/host-meta", get(host_meta))
        .route("/.well-known/webfinger", get(webfinger))
        .route("/actor", get(person_handler))
        .layer(
            ServiceBuilder::new()
                .layer(NormalizePathLayer::trim_trailing_slash())
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::new(std::time::Duration::from_secs(10))),
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
