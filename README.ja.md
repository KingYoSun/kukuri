[English](./README.md) | 日本語

# kukuri

kukuri は、興味のある話題から人やコミュニティにつながる、トピック中心の P2P ソーシャルアプリです。公開の会話に参加したり、同じ話題の中で小さな非公開チャンネルへ移ったりしながら、自分の端末を基点にアイデンティティを保持できます。

![トピックのタイムラインと返信スレッドを表示した kukuri デスクトッププレビュー](./docs/assets/readme/kukuri-desktop-preview.png)

## Builder Preview をダウンロードする

> [!IMPORTANT]
> 現在の kukuri は、テスター向けの **Builder Preview** です。一般公開の安定版ではありません。

**[最新の Windows プレビューをダウンロード](https://github.com/KingYoSun/kukuri/releases/latest)**

| 環境 | 現在の対応状況 |
| --- | --- |
| Windows 10 / 11 | 最新の GitHub Release から NSIS インストーラーを配布 |
| Linux | ソースから起動。インストーラーは未提供 |
| macOS | 現在パッケージは未提供 |

プレビューのインストーラーは未署名の場合があります。その場合は Windows SmartScreen の警告が表示されることがあるため、実行前にリリースノートを確認してください。

詳しいセットアップと復旧方法は、[利用者向けクイックスタート](./docs/runbooks/mvp-user-quickstart.md)と[トラブルシューティング](./docs/runbooks/mvp-troubleshooting.md)を参照してください。

## kukuri でできること

- 興味のあるトピックを見つけ、投稿や返信でスレッド形式の会話に参加できます。
- コミュニティを別々の場所へ分断せず、同じトピック内で公開投稿と非公開チャンネルを使い分けられます。
- フォロー、リアクション、再投稿、引用、ブックマークを利用し、見たくない投稿者は自分の端末上でミュートできます。
- 相互につながった相手と DM を交わし、画像や動画を共有できます。
- 返信、メンション、フォロー、再投稿、メッセージをアプリ内通知や OS 通知で確認できます。
- 再起動、一時的なオフライン、プレビュー版の更新を挟んでも、アイデンティティとローカルの状態を引き継げます。

## 3 分で試す

1. Windows プレビューをインストールして起動するか、[Linux でソースから起動](#開発クイックスタート)します。
2. あらかじめ設定された Community Node が `ready` になるまで数秒待ち、最初から用意されたトピックを開きます。
3. 公開投稿を 1 件作成し、既存の投稿へ返信します。
4. 同じトピックの中で非公開チャンネルを作成するか、既存のチャンネルへ参加します。
5. `Settings -> Release` から診断レポートを書き出し、GitHub へフィードバックを送ります。

既定の診断レポートには、秘密鍵、認証トークン、非公開チャンネルの秘密情報、招待・共有トークン、DM 本文、ローカルデータベースのパスを含めません。

## プレビューの状態と制限

- パッケージ版プレビューの現在の対象は Windows 10 / 11 のみです。Linux はソース起動、macOS はパッケージ未提供です。
- DM の確認には、別のテスト用ピアと相互関係が必要です。P2P の動作は、2 台の端末またはデータ領域を分けた 2 つのアプリで確認しやすくなります。
- Live、Metaverse、ゲームルームの画面は現在も発展途上です。一部の拡張機能は開発者モードの段階にあり、Stream にはまだメディアプレーヤーがありません。
- プレビュー版の更新では、アイデンティティ、ローカルデータベース、Iroh のデータ、Community Node の設定、非公開チャンネルの権限情報、通知一覧を保持する前提です。ローカル状態を残したい場合は、アンインストールやリセットの前にアプリのデータディレクトリを保管してください。
- テスト中のソフトウェアです。接続、更新、復旧の問題を報告するときは、秘匿情報を除去した診断レポートを添付してください。

現在のマイルストーンは [Builder Preview 計画](./docs/progress/2026-04-16-mvp-builder-preview-plan.md)、配布とデータ安全性の検査は[リリース手順書](./docs/runbooks/release.md)を参照してください。

## kukuri の仕組み

- **アイデンティティは利用者のものです。** 署名鍵はローカルに保存します。Community Node はアカウントの所有者でもホームサーバーでもありません。
- **P2P が基盤です。** 通信は Direct P2P、Relay Supported P2P の順に優先し、それらでデータを運べない場合だけ Relay Fallback を使います。
- **Community Node の支援範囲は限定されています。** ノードは初期接続、認証、トピックの合流支援、接続、索引、モデレーション、通報などを補助できます。一方、利用者の投稿、プロフィール、ソーシャルグラフの恒久的な正本ではなく、ネットワーク全体への権限も持ちません。
- **データの種類ごとに経路を分けます。** 構造化された共有状態は `docs`、メディアや大きなデータは `blobs` で同期します。`hints` は同期が必要かもしれないことをピアへ知らせるだけです。
- **Nostr 互換は意図的に限定しています。** アイデンティティ、署名付きエンベロープ、一部タグの有用な意味づけを利用しますが、完全な Nostr クライアントではなく、内部同期もリレー優先ではありません。
- **モデレーションの効力は発行元の範囲に閉じます。** モデレーションイベントや安全性の勧告は、発行したノードからの任意の信頼情報であり、ネットワーク全体への命令ではありません。適用方法は各クライアントが判断します。

恒久的な責任境界は [P2P-first Community Node の責任境界](./docs/architecture/p2p-first-community-node-responsibility-boundary.md)に記載しています。

## 現在利用できる範囲

| 分野 | 現在の Builder Preview で利用できる機能 |
| --- | --- |
| トピックと投稿 | トピックの検索・絞り込み、公開投稿、返信とスレッド、リアクション、再投稿、引用、ブックマーク、ローカルミュート |
| 非公開の会話 | `invite_only`、`friend_only`、`friend_plus` の各ポリシーと世代更新に対応したチャンネル、相互関係に限定した 1 対 1 の DM |
| 人と活動 | 公開プロフィール、フォローと解除、相互関係と友達の友達という文脈、アプリ内通知と OS 通知 |
| メディア | 投稿と DM への画像・動画添付 |
| 接続と復旧 | 静的ピア、シード情報を使った DHT 探索、Community Node による接続支援、オフライン対応のローカル状態、再起動からの復元、後から参加したピアへの履歴補完 |
| プレビュー運用 | アプリ内の更新確認、秘匿情報除去済みの診断、フィードバックへのリンク、来歴表示、分散通報ルーティング |

出荷済み機能の詳細な基準は、[基盤の進捗記録](./docs/progress/2026-03-10-foundation.md)、承認済みの [ADR](./docs/adr/)、テスト、[ハーネスのシナリオ](./harness/scenarios/)を正とします。

## 今後の方向性

- 検索、発見、推薦、ゲートウェイ、ブリッジは、P2P の中核に必須の正本を置かず、任意のサービスとして拡張できます。
- Community Node の信頼、モデレーション、ポリシー支援、運用者向け機能は、各ノードが宣言した能力と権限の範囲内で発展させられます。
- Live、Metaverse、ゲーム、より豊かなメディア体験は、トピック中心の所有権と同期境界を変えずに成熟させていきます。

これらは方向性であり、現在のプレビューですべて利用できるという約束ではありません。

## フィードバックとコミュニティ

- 再現可能な不具合やリグレッションは [GitHub Issues](https://github.com/KingYoSun/kukuri/issues) へ報告してください。
- 質問、製品アイデア、UX 提案、大きな変更の事前相談には [GitHub Discussions](https://github.com/KingYoSun/kukuri/discussions) を利用してください。
- 接続、更新機能、復旧の問題には、`Settings -> Release` から取得した秘匿情報除去済みレポートを添付してください。
- Community Node 運用者からのデプロイ、情報開示、モデレーション、分散通報に関するフィードバックも、同じ GitHub の窓口で受け付けます。

## コントリビューション

コード以外の貢献も歓迎します。不具合報告、UI/UX 提案、文書、翻訳、テスト、実装、Community Node 運用上のフィードバックはいずれもプロジェクトの助けになります。

大きな機能、プロトコル変更、責任境界の変更、大規模なリファクタリングは、実装前に Discussion で相談してください。不具合は Issue に焦点を絞って報告し、振る舞いの正本にはリポジトリ内のテストと文書を使ってください。

### 開発クイックスタート

必要な環境:

- Git
- `rust-toolchain.toml` で固定された Rust `1.92.0`
- Node.js `^20.19.0` または `>=22.12.0`
- 以下のコマンドから利用する pnpm `10.16.1`
- [開発手順書](./docs/runbooks/dev.md)に記載された環境別の依存パッケージ。Windows 開発には [Tauri の事前要件](https://v2.tauri.app/start/prerequisites/#windows) も必要です
- Docker は Community Node の統合テストとローカル Community Node 構成を使う場合のみ必要です

```bash
git clone https://github.com/KingYoSun/kukuri.git
cd kukuri

npx pnpm@10.16.1 install --dir apps/desktop
cargo xtask doctor

cd apps/desktop
npx pnpm@10.16.1 tauri:dev
```

通常の検査はリポジトリのルートで実行します。

```bash
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke
```

ブラウザー上のフロントエンドだけを扱う場合は `npx pnpm@10.16.1 --dir apps/desktop dev` を利用できます。対象別の検査、UI 検証、Community Node の作業手順、環境ごとの設定は[開発手順書](./docs/runbooks/dev.md)にまとめています。

## 文書

- [文書索引](./docs/README.md)
- [Builder Preview 計画](./docs/progress/2026-04-16-mvp-builder-preview-plan.md)
- [基盤と出荷済み機能の基準](./docs/progress/2026-03-10-foundation.md)
- [利用者向けクイックスタート](./docs/runbooks/mvp-user-quickstart.md)
- [トラブルシューティング](./docs/runbooks/mvp-troubleshooting.md)
- [開発手順書](./docs/runbooks/dev.md)
- [リリース手順書](./docs/runbooks/release.md)
- [P2P-first Community Node の責任境界](./docs/architecture/p2p-first-community-node-responsibility-boundary.md)
- [アーキテクチャ判断記録](./docs/adr/)
- [サードパーティー通知](./docs/THIRD_PARTY_NOTICES.md)

## ライセンス

[MIT](./LICENSE)
