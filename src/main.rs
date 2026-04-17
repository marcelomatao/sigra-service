use tokio::net::TcpListener;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    fmt().with_env_filter(EnvFilter::from_default_env()).json().init();

    let app = sigra_service::app().await?;

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("sigra-service listening on {addr}");

    axum::serve(listener, app).await?;
    Ok(())
}
