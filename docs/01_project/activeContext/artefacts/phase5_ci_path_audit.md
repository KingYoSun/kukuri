# Phase 5 CI/ローカルスクリプト パス依存調査
最終更新日: 2025年10月20日

| 対象 | 現状参照パス／コマンド | 影響範囲 | 修正案 |
| --- | --- | --- | --- |
| `scripts/docker/run-smoke-tests.sh` | `cargo test --package kukuri-tauri --lib modules::p2p::tests::iroh_integration_tests::`<br>`cargo test --package kukuri-tauri --lib modules::p2p::tests::mainline_dht_tests::` | Docker スモークテスト。Rust テストパスが `modules::p2p::tests::*` に固定されているため、Phase 5 で `tests/integration/p2p/*` へ移動すると失敗する。 | テスト移行後は `cargo test --package kukuri-tauri --test p2p_mainline_smoke -- --nocapture` 等、`tests/` 配下の新モジュール名に更新する。併せて Runbook の手順も改訂。 |
| `scripts/test-docker.sh` | 既定フィルター `modules::p2p::tests::iroh_integration_tests::` | ローカル Docker P2P テスト (`./scripts/test-docker.sh p2p`) が旧モジュール名に依存。 | Phase 5 後の新しいテストモジュール名（例: `tests::integration::p2p::iroh::`) を `TESTS` 既定値に設定し、ヘルプ文言も更新。 |
| `scripts/test-docker.ps1` | `cargo test --package kukuri-tauri --lib modules::p2p::tests::iroh` | Windows 用 Docker テスト。`--lib` 指定で旧モジュールを直接参照。 | テスト移行後は `--test` オプションに変更し、新しいファイル名へ差し替える。PowerShell 側のログ文言も更新する。 |
| `docker-compose.test.yml` (`test-runner` / `rust-test`) | ボリューム: `./kukuri-tauri/src-tauri/src` のみマウント（`src-tauri/tests` はイメージ内） | Phase 5 で Rust テスト資産を `src-tauri/tests` へ集約すると、ローカル変更がコンテナに反映されず再ビルドが必要。 | `./kukuri-tauri/src-tauri/tests:/app/kukuri-tauri/src-tauri/tests:ro` を追加し、ローカル編集を即座に反映させる。`test-runner` と `rust-test` の双方を更新する。 |
| GitHub Actions `test.yml`（Docker ジョブ） | `/app/run-tests.sh` → `cargo test --workspace`（再帰的に `modules::p2p::tests::*` を呼ぶ） | `run-tests.sh` が Phase 5 更新後の `cargo test` を利用できるようにする必要がある。 | `run-tests.sh` 内部で個別モジュールパスを指定しないため直接の修正は不要だが、P2P スモーク用途で `modules::p2p::tests::*` を参照するステップ追加時は新構成に合わせる。 |
