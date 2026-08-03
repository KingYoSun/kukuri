//! cn-indexer binary（#413）。
//!
//! community node の docs replica sync participant（Model C）を起動する。起動時に relay validation
//! gate（ADR 0025 §6.4）を適用し、自前 relay も外部 relay も無ければ起動しない（fail-closed）。
//!
//! `validate-config` 引数（#614）を渡すと、外部接続なしの起動前検証（relay gate / channel
//! secret 鍵 / safety provider 解決 / scan service 構成）だけを行って終了する。image smoke /
//! operator の構成確認用。

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => kukuri_cn_indexer::run_from_env().await,
        Some("validate-config") => kukuri_cn_indexer::validate_config_from_env(),
        Some(other) => {
            anyhow::bail!("unknown argument `{other}` (usage: cn-indexer [validate-config])")
        }
    }
}
