use anyhow::Result;

#[allow(unused_imports)]
use crate::*;

const GENERATED_TS: &str = "apps/desktop/src/lib/api/types.generated.ts";

/// Rust の IPC 型から `types.generated.ts` を再生成する(WP-H7 PR3)。
///
/// - `cargo test -p kukuri-desktop-runtime --features ts export_ipc_types` で TS を書き出す
///   (生成器は H7 Stage3b で app-api から desktop-runtime へ移設済み。
///   `TS_RS_LARGE_INT=number` で u64/i64 を number にする)。
/// - リポジトリに prettier 等のフォーマッタは無く、出力は ts-rs の決定論的な
///   raw 形式のまま採用する(機械生成ファイル。手編集しない)。
/// - `check = true`(CI)のときは再生成後に `git diff --exit-code` で
///   Rust と TS の乖離(手動同期漏れ)を検出する。
pub(crate) fn ipc_types(check: bool) -> Result<()> {
    run_with_env(
        "cargo",
        [
            "test",
            "-p",
            "kukuri-desktop-runtime",
            "--features",
            "ts",
            "export_ipc_types",
        ],
        &root_dir(),
        &[("TS_RS_LARGE_INT", "number")],
    )?;

    if check {
        run(
            "git",
            ["diff", "--exit-code", "--", GENERATED_TS],
            &root_dir(),
        )?;
    }

    Ok(())
}
