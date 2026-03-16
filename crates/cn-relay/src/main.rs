use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    kukuri_cn_relay::run_from_env().await
}
