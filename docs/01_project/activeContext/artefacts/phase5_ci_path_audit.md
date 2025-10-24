# Phase 5 CI/ローカルスクリプト パス依存調査
最終更新日: 2025年10月22日

| 対象 | 現状参照パス／コマンド | 影響範囲 | 修正案 |
| --- | --- | --- | --- |
| `scripts/docker/run-smoke-tests.sh` | `cargo test --package kukuri-tauri --test p2p_gossip_smoke -- --nocapture --test-threads=1`<br>`cargo test --package kukuri-tauri --test p2p_mainline_smoke -- --nocapture --test-threads=1` | Docker スモークテストで新テストバイナリを並列実行。 | 2025年10月22日: フォールバックロジックを撤廃し、新バイナリを常に実行する構成へ更新済み。 |
| `scripts/test-docker.sh` | 既定フィルター `p2p_gossip_smoke`（`--tests mainline` で切替可） | ローカル Docker P2P テスト (`./scripts/test-docker.sh p2p`) が新テスト名に追随。 | 2025年10月22日: フォールバック削除・エイリアス追加を完了し、旧スモークバイナリへの依存を解消。 |
| `scripts/test-docker.ps1` | `cargo test --package kukuri-tauri --test p2p_gossip_smoke`<br>`cargo test --package kukuri-tauri --test p2p_mainline_smoke` | Windows 用 Docker テストで2つのスモークを順次実行。 | 2025年10月22日: 新バイナリ固定化とログ整備を完了。 |
| `docker-compose.test.yml` (`test-runner` / `rust-test`) | ボリューム: `./kukuri-tauri/src-tauri/src` のみマウント（`src-tauri/tests` はイメージ内） | Phase 5 で Rust テスト資産を `src-tauri/tests` へ集約すると、ローカル変更がコンテナに反映されず再ビルドが必要。 | `./kukuri-tauri/src-tauri/tests:/app/kukuri-tauri/src-tauri/tests:ro` を追加し、ローカル編集を即座に反映させる。`test-runner` と `rust-test` の双方を更新する。 |
| GitHub Actions `test.yml`（Docker ジョブ） | `/app/run-tests.sh` → `cargo test --workspace`（`tests/` 配下の新バイナリを網羅実行） | `run-tests.sh` が Phase 5 更新後の `cargo test` を利用できるようにする必要がある。 | `run-tests.sh` 内で個別モジュールパスを指定しないため直接の修正は不要だが、P2P スモークを追加する際は `--test p2p_gossip_smoke` / `--test p2p_mainline_smoke` を明示する。 |
