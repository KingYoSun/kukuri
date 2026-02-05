# クライアントKIP検証強化
日付: 2026年02月05日

## 概要
- CommunityNodeHandler の KIP 検証で k/ver/必須タグ/schema を追加し、不正イベントを除外。
- ラベル/アテステーション/ノード広告の検証テストを拡充。

## 対応内容
- validate_kip_event_json に k/ver/必須タグ/schema 判定を追加（label/attestation/node descriptor/topic service/trust anchor）。
- テスト用イベント生成を KIP 必須タグ + schema に更新。
- 不正イベント拒否のユニットテストを追加（k/ver 欠落、policy 欠落、schema 不一致）。

## 検証
- `./scripts/test-docker.ps1 rust`（ENABLE_P2P_INTEGRATION 未設定で一部 skip）
- `gh act --workflows .github/workflows/test.yml --job format-check`（git clone の some refs were not updated / pnpm approve-builds 警告）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（git clone の some refs were not updated / pnpm approve-builds 警告 / act・useRouter 警告）
