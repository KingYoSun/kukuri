# Access Control P2P-only relay/DB 整合

日付: 2026年02月06日

## 概要
- cn-relay の `scope!=public` で DB membership/epoch 検証を撤去し、P2P-only 方針に合わせた最小検証へ整理。
- relay/DB の役割分担（検証はクライアント側、ノード保持は v1 必須ではない）をドキュメントへ反映。

## 実施内容
- `cn-relay` の ingest で `epoch` タグの必須/整数/正数チェックのみを実施。
- WS 配信側の private scope 判定も `epoch` タグの最小検証に統一。
- `docs/03_implementation/community_nodes/access_control_design.md` に P2P-only の運用メモを追記。

## 検証
- `./scripts/test-docker.ps1 rust`
- `docker run --rm --network kukuri-community-node_cn -e DATABASE_URL=postgres://cn:cn_password@postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.86-bookworm bash -c "cargo test --locked -p cn-relay"`
- `docker run --rm -v C:\Users\kgm11\kukuri\kukuri-cli:/app -w /app rust:1.86-bookworm cargo test --locked`
- `gh act --workflows .github/workflows/test.yml --job format-check`（ログ: `tmp/logs/gh-act-format-check-20260206-122344.log` / 警告: some refs were not updated / pnpm approve-builds）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（ログ: `tmp/logs/gh-act-native-test-linux-20260206-122517.log` / 警告: some refs were not updated / pnpm approve-builds / useRouter 警告）

## 補足
- `./scripts/test-docker.ps1 coverage` は 20/30/30 分でタイムアウト（未完了）。
