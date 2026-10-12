// src/main.rs
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    mailrust::cli::run().await
}
