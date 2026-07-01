//! cn-indexer binary（#413）。
//!
//! community node の docs replica sync participant（Model C）を起動する。起動時に relay validation
//! gate（ADR 0025 §6.4）を適用し、自前 relay も外部 relay も無ければ起動しない（fail-closed）。

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    kukuri_cn_indexer::run_from_env().await
}
