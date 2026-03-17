# ADR 0007: Windows Desktop Support

## Status
Accepted

## Feature Data Classification
- Feature 名: Windows desktop support
- Durable / Transient: Durable
- Canonical Source: 既存 current implementation と同じく `docs + blobs + gossip hints + SQLite projection + local identity storage`
- Replicated?: Yes
- Rebuildable From: 既存 feature ごとの canonical source と local identity backend
- Public Replica / Private Replica / Local Only: 既存 feature ごとの replica 区分を維持し、Windows 対応で新しい replica は増やさない
- Gossip Hint 必要有無: 既存 feature ごとの要件を維持し、Windows 対応だけでは新規 hint を増やさない
- Blob 必要有無: 既存 feature ごとの要件を維持する
- SQLite projection 必要有無: 既存 feature ごとの要件を維持する
- 必須 contract:
  - `auto_mode_prefers_keyring_secret_over_file_secret`
  - `auto_mode_falls_back_to_file_when_keyring_write_fails`
  - `auto_mode_generated_keyring_secret_survives_restart`
  - `file_only_mode_rejects_existing_keyring_backend_marker`
  - `cargo xtask check` で Tauri backend compile が通ること
  - `cargo xtask desktop-package` が Windows 上で NSIS artifact を生成できること
- 必須 scenario:
  - Windows native smoke として `post -> restart -> persist -> static-peer sync -> live create/join/end -> game create/update` を確認する

## Decision
- Windows 対応は現行 `static-peer` 前提 desktop feature set の parity に限定し、DHT discovery と community-node connectivity/auth は含めない。
- Windows の local identity backend は Credential Manager を標準にし、利用できない場合のみ既存 file backend へ fallback する。
- backend marker の値は `keyring` / `file` を維持し、platform 別の追加 marker は導入しない。
- Tauri の Windows packaging は別 overlay config で扱い、v1 の配布物は current-user NSIS installer のみを正式サポートにする。

## Consequences
- Windows 対応のために docs/blobs/gossip/store の canonical source や replica 責務を変えてはならない。
- Windows native keyring failure は既存 file fallback で吸収し、author identity の永続性は維持しなければならない。
- Windows CI lane は non-required で導入し、Linux required lane の determinism は維持する。
