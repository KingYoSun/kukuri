# Issue #40 マルチレイヤーライセンス監査（audit-first）

作成日: 2026年02月16日
最終更新日: 2026年02月16日

## 監査前提

- リポジトリ方針（MIT）:
  - `LICENSE` 冒頭が `MIT License`。
  - `README.md:168` が `MIT. See [LICENSE](./LICENSE).` を明示。
  - `README.ja.md:168` が `MIT。詳細は [LICENSE](./LICENSE) を参照してください。` を明示。
- 監査対象レイヤー（Manager 指示）:
  1. ルート LICENSE/COPYING
  2. サブディレクトリ LICENSE/COPYING
  3. Rust `Cargo.toml` (`license` / `license-file`)
  4. Node `package.json` (`license`)
  5. README/docs の明示ライセンス宣言
- 実査対象は Git 管理下ファイルに限定（`git ls-files` ベース）。依存配下 `node_modules/**` は除外。

## 実行コマンド

- `find . -maxdepth 1 -type f \( -iname 'license*' -o -iname 'copying*' \) | sort`
- `git ls-files | rg -i '(^|/)(license[^/]*|copying[^/]*)$' | sort`
- `rg -n 'license' --glob '**/Cargo.toml'`
- `git ls-files '**/package.json' | sort | while read -r f; do jq ... "$f"; done`
- `rg -n -i '\blicen[cs]e\b|ライセンス' README* kukuri-community-node/README* kukuri-tauri/README* docs/**/*.md`
- `sed -n '1,220p' kukuri-community-node/Cargo.toml`
- `for f in $(git ls-files 'kukuri-community-node/crates/*/Cargo.toml'); do sed -n '1,40p' "$f"; done`

## レイヤー別監査結果

| レイヤー | 証跡 | 判定 |
|---|---|---|
| 1) ルート LICENSE/COPYING | `LICENSE` のみ存在、内容は MIT | 整合 |
| 2) サブディレクトリ LICENSE/COPYING | Git 管理下に追加の LICENSE/COPYING なし | 整合（矛盾なし） |
| 3) Rust metadata | `kukuri-community-node/Cargo.toml:19` が `license = "Apache-2.0"`、各 crate は `license.workspace = true` で継承 | **不一致** |
| 4) Node metadata | `kukuri-community-node/apps/admin-console/package.json` と `kukuri-tauri/package.json` に `license` フィールドなし | 未宣言 |
| 5) README/docs 明示宣言 | `README.md` / `README.ja.md` は MIT 明示。`docs/03_implementation/iroh_v090_specification.md:232-234` は iroh 依存ライセンス情報（第三者コンポーネント） | ルート方針は整合 |

## MIT 方針との不一致マップ

### M-01: Rust ワークスペースライセンス競合（重大）

- 対象: `kukuri-community-node/Cargo.toml:19`
- 現状: `license = "Apache-2.0"`
- 期待: `MIT`
- 波及: 下記 10 crate が `license.workspace = true` で `Apache-2.0` を継承
  - `kukuri-community-node/crates/cn-admin-api/Cargo.toml:5`
  - `kukuri-community-node/crates/cn-bootstrap/Cargo.toml:5`
  - `kukuri-community-node/crates/cn-cli/Cargo.toml:5`
  - `kukuri-community-node/crates/cn-core/Cargo.toml:5`
  - `kukuri-community-node/crates/cn-index/Cargo.toml:5`
  - `kukuri-community-node/crates/cn-kip-types/Cargo.toml:5`
  - `kukuri-community-node/crates/cn-moderation/Cargo.toml:5`
  - `kukuri-community-node/crates/cn-relay/Cargo.toml:5`
  - `kukuri-community-node/crates/cn-trust/Cargo.toml:5`
  - `kukuri-community-node/crates/cn-user-api/Cargo.toml:5`

### M-02: Node `license` 未宣言（中）

- 対象1: `kukuri-community-node/apps/admin-console/package.json`
  - 現状: `license` フィールドなし
  - 期待: `"license": "MIT"`
- 対象2: `kukuri-tauri/package.json`
  - 現状: `license` フィールドなし
  - 期待: `"license": "MIT"`
- 備考: 明示的な非MIT宣言はないが、リポジトリ方針の伝播が不足。

## 最小 remediation plan（1タスク=1PR）

1. PR-1（Issue #40 本体）: Rust ライセンス競合を解消
   - 変更対象: `kukuri-community-node/Cargo.toml`
   - 変更内容: `[workspace.package].license` を `Apache-2.0` から `MIT` へ変更
   - 期待効果: community-node 配下 crate の `license.workspace = true` 継承が一括で MIT に揃う

2. PR-2（Issue #40 追補）: Community Node の Node metadata を MIT 明示
   - 変更対象: `kukuri-community-node/apps/admin-console/package.json`
   - 変更内容: `"license": "MIT"` を追加
   - 期待効果: community-node サブツリーの Node メタデータを MIT 方針に整合

3. PR-3（リポジトリ横断整備・任意）: Tauri 側 Node metadata を MIT 明示
   - 変更対象: `kukuri-tauri/package.json`
   - 変更内容: `"license": "MIT"` を追加
   - 期待効果: リポジトリ全体の Node メタデータ整合を完成

## 結論

- Issue #40 の audit-first 監査は完了。
- 主要不一致は `kukuri-community-node/Cargo.toml` の `Apache-2.0` で、これが最優先修正点。
- 次実装タスクは PR-1（Rust workspace license の MIT 化）。
