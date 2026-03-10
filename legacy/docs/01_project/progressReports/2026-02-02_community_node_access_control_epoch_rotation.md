# Community Node Access Control epochローテ/追放フロー実装

作成日: 2026年02月02日

## 概要
- community node 側で epoch ローテ/追放を実行し、残留者への key.envelope 再配布を行うフローを実装した。
- ローテ/追放を Admin API と CLI から実行できるようにした。

## 実施内容
- cn-core に Access Control サービスを追加し、epoch++ と key.envelope 再配布、メンバー追放処理を実装。
- cn-admin-api に rotate/revoke エンドポイントを追加し、監査ログに記録。
- cn-cli に rotate/revoke コマンドを追加し、JSON で結果を出力。
- cn-user-api の key.envelope 発行処理を共通実装へ移行。

## 技術的詳細
- cn_admin.topic_scope_state の current_epoch をインクリメントし、新 epoch の群鍵を生成・保存。
- cn_user.topic_memberships の active メンバーへ新 epoch の key.envelope を再生成して cn_user.key_envelopes に upsert。
- revoke 時は membership を revoked に更新した上でローテ処理を実行。

## 次のステップ
- access_control を P2P-only とする方針の整理（User API との矛盾解消）。
- Admin Console への Access Control 画面組み込み。

## 課題・懸念事項
- 大規模メンバー数のローテ時にトランザクションが長くなる可能性。

## まとめ
Access Control の epochローテ/追放を community node 側で完結できるようになり、CLI と Admin API から運用可能になった。
