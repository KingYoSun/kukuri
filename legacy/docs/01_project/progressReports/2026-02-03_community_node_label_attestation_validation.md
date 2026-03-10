# Community Node label/attestation クライアント検証

- Tauri の Community Node 取得結果に署名/exp/採用ノードの検証を追加し、未採用ノード由来の label/attestation を除外
- trust スコア集約は有効な attestation を持つノードのみ採用するように変更
- 検証ロジックのユニットテストを追加
