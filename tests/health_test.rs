//! Integration tests — requires MongoDB on localhost:27017 and MinIO on localhost:9000.

use reqwest::StatusCode;
use tokio::net::TcpListener;

async fn start_server() -> std::net::SocketAddr {
    // SAFETY: single-threaded test setup before server starts; no concurrent env reads.
    unsafe {
        std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
        std::env::set_var("MONGODB_DATABASE", "sigra_test");
        std::env::set_var("S3_BUCKET", "sigra-test");
        std::env::set_var("S3_ENDPOINT", "http://localhost:9000");
        std::env::set_var("S3_REGION", "us-east-1");
        std::env::set_var("EAS_RPC_URL", "https://mainnet.base.org");
        std::env::set_var(
            "EAS_PRIVATE_KEY",
            "0000000000000000000000000000000000000000000000000000000000000001",
        );
        std::env::set_var(
            "EAS_SCHEMA_UID",
            "0x0000000000000000000000000000000000000000000000000000000000000000",
        );
        // Large interval so the anchor loop never fires during tests.
        std::env::set_var("ANCHOR_INTERVAL_SECS", "99999");
    }

    let app = sigra_service::app().await.expect("build app");
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    addr
}

#[tokio::test]
async fn health_returns_ok() {
    let addr = start_server().await;
    let resp = reqwest::get(format!("http://{addr}/health")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn missing_user_header_returns_403() {
    let addr = start_server().await;
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/api/documents"))
        .header("content-type", "application/json")
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
