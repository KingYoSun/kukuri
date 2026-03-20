# Windows MSVC: VERSION リソース重複（CVT1100 / LNK1123）と `crate-type` 調整

**最終更新**: 2026年03月19日

## 概要（事象）

Windows（MSVC）で `kukuri-tauri` の `cargo test` / 各種バイナリをリンクする際、次のようなエラーが出る場合がある。

- `CVTRES : fatal error CVT1100`（`VERSION` リソースの重複）
- `LINK : fatal error LNK1123`（COFF 変換失敗として続くことがある）

## 原因

1. **`tauri-build`（Windows）** は内部で `tauri_winres` を用い、**一度** `resource.lib` を生成し、`cargo:rustc-link-lib=dylib=resource` を出力する。ここには **VERSION** と **RT_MANIFEST**（既定で Common Controls v6 依存の記述を含む）が含まれる。
2. 同じ `build.rs` で **`winres::WindowsResource::compile()` をもう一度**呼ぶと、同じ種類の **VERSION** が再度リンク対象となり、MSVC のリソースマージで **重複** と判定される。

※ `origin/main`（調査時点）の `build.rs` には上記の「二重 `winres`」コードは**含まれていなかった**。再現した環境では、ローカル変更や別ブランチで `winres` が追加されていた可能性がある。**再発防止**のため `build.rs` に短い注意コメントを残す。

## 実施した修正（本件のスコープ）

| 対象 | 内容 | `main` からの実質的な挙動差 |
|------|------|-----------------------------|
| `kukuri-tauri/src-tauri/Cargo.toml` | `[lib] crate-type` から `staticlib` を削除し `["cdylib", "rlib"]` とする | **あり**（Windows での `staticlib` リンク負荷・テスト時の扱いが変わる） |
| `kukuri-tauri/src-tauri/build.rs` | `tauri_build::build()` のみ。二重 `winres` を足さない旨の短いコメント | `main` では元々 `winres` なし → **コメント追加が主** |
| `docs/03_implementation/testing_guide.md` | Windows 節に CVT1100 と `STATUS_ENTRYPOINT_NOT_FOUND` の注記を追加 | ドキュメントのみ |

### `crate-type` から `staticlib` を外す理由（要約）

- Tauri デスクトップ向けには `cdylib` が必要。
- 単体テスト・他クレートからの利用には `rlib` が使われる。
- `staticlib` は C 静的リンク用途が主で、Windows で裸 `cargo test` 時に余計なリンク経路を踏みやすい。**静的 FFI を本プロジェクトが要求しない限り**外してよい、という整理。

## 本件とは別の変更（混在時の整理）

次は **CVT1100 修正とは独立**した変更として、別コミット / 別 PR に分けることを推奨する。

| パス / 要素 | 内容 |
|-------------|------|
| `presentation/handlers/community_node_handler.rs`（テストモジュール） | Windows では `dirs::data_dir()` が `APPDATA` を参照するため、テスト用環境変数を `XDG_DATA_HOME` ではなく **`APPDATA`** に切り替える `cfg` 対応。**データディレクトリまわりのテスト正しさ**向け。 |
| `tauri-win-link-test/`（リポジトリ直下） | 最小 Tauri でリンク・`cargo test` を切り分け検証するための**サンプル**。本番修正の**必須構成ではない**。コミットする場合は目的を README に明記するか、`.gitignore` するか方針を決める。 |
| `todo.md` | 作業メモならコミット対象外が無難。 |
| `Cargo.lock` | `crate-type` のみの変更では通常パッケージ解決は変わらない。**改行 CRLF/LF のみの差分**になっていないか、コミット前に `git diff` で確認すること。 |

## 検証

- **リンクまで**: `cd kukuri-tauri/src-tauri && cargo build --lib` が MSVC で成功すること。
- **テスト実行（Windows ホスト）**: Tauri / ネイティブ依存により `STATUS_ENTRYPOINT_NOT_FOUND` が出る場合がある。リポジトリ方針どおり **`./scripts/test-docker.ps1 rust`** 等での検証を推奨（`AGENTS.md` / `testing_guide.md` 参照）。

## 参考

- `tauri-build` 既定の Windows マニフェスト（Common Controls v6）: クレート内 `windows-app-manifest.xml`（`tauri-build` ソース参照）
- 本リポジトリのテスト方針: `docs/03_implementation/testing_guide.md`（「Windows環境特有の問題」）
