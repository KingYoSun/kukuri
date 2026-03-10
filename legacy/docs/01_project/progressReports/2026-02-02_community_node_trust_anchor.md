# Community Node Trust Anchor (39011) 実装

作成日: 2026年02月02日

## 概要
- trust.anchor(39011) の発行/保存/取得/クリアと、UI からの attester 切替に連動した trust 表示更新を実装した。
- PostCard の trust バッジが採用 attester に応じて切替わることを確認できる状態にした。

## 実施内容
- Tauri 側に trust.anchor(39011) の発行・保存・取得・削除のコマンドを追加。
- フロントの Community Node 設定に attester 選択 UI を追加し、選択変更で trust 表示を更新。
- PostCard の trust 判定に採用 attester を反映し、バッジ表示に attester 表示を付与。
- unit テストとモックを更新し、trust anchor 適用の挙動を検証。

## 技術的詳細
- Tauri handler で Nostr event(kind=39011) を生成して保存し、attester/weight/タグを検証。
- `community_node_get_trust_anchor` / `community_node_set_trust_anchor` / `community_node_clear_trust_anchor` を追加。
- フロントの queryKey に attester を含め、切替時の再取得を保証。

## 次のステップ
- Access Control の epoch ローテ/追放フロー実装。
- trust/label/attestation のクライアント側検証（署名/exp/採用ノード）を追加。
- trust.anchor の E2E 手順を追加して attester 切替の実地確認を自動化。

## 課題・懸念事項
- vitest 実行時に既知の act/useRouter 警告が発生（失敗はなし）。

## まとめ
trust.anchor(39011) の発行/保存/適用/UI を揃え、採用 attester 切替で trust 表示が変わる経路を完成させた。