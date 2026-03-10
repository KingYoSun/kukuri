# GitHub Actions 失敗対応（Community Node Tests）

- cn-kip-types の `validate_key_envelope_accepts_valid_event` で self-tagging により `p` タグが落ちる前提を避けるため、署名鍵と受信者鍵を分離
- cn-user-api のテスト用ルートを axum v0.8 形式の `{topic_id}` に修正
- `gh act` で `community-node-tests` / `format-check` / `native-test-linux` を再実行し成功（Vitest の act/useRouter 警告は既知）