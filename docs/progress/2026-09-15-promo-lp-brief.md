# LP・告知素材の共通brief（Issue #1037）

## この文書の位置づけ

- 所有Issue: #1037（統括 #1036）。本書は後続 #1039 / #1040 / #1041 / #1042 / #1043 / #1044 が参照する共通briefの正本。
- Scope revision: 2026-09-15-promo-lp-brief-v2
- 作業日: 2026-09-18
- 作業時の main: `d455bdd8e03541de9734f06a8e050a72bdb01bfd`
- 配布候補release: `v0.2.5-preview.3`（2026-09-17公開）。Issue起票時点の記載 `v0.2.3-preview.2` から更新した（2026-09-18にユーザーが候補を確定）。
- 配布候補commit: `05820f976e94dfa5cd985c7e0bc5a8ce84420af8`（tag `v0.2.5-preview.3` のpeel先。main の祖先であることを確認済み）
- 本書は製品仕様の正本ではない。仕様は `docs/adr/`、実行手順は `docs/runbooks/`、視覚契約は `DESIGN.md` を正本とし、本書はそれらを素材制作の条件へ翻訳したものとする。

## AC-1: 配布候補の事実確認

### release と配布物

出典: `gh release view v0.2.5-preview.3`（2026-09-18取得）、[release一覧](https://github.com/kukuri-app/kukuri/releases/latest)、`CHANGELOG.md`、`README.md`。

| 事実 | 値 | 出典 |
| --- | --- | --- |
| tag | `v0.2.5-preview.3` | GitHub Release |
| 公開日時 | 2026-09-17T10:10:14Z | GitHub Release |
| commit | `05820f976e94dfa5cd985c7e0bc5a8ce84420af8` | `refs/tags/v0.2.5-preview.3^{}` |
| 表示バージョン | `0.2.5`（asset名）／`v0.2.5-preview.3`（tag） | asset名とtag |
| channel | preview | release notes「Preview channel: preview」 |

配布asset（素材に載せてよい配布物はこの一覧に限る）。

| 対象 | asset | 備考 |
| --- | --- | --- |
| Windows 10 / 11 | `kukuri_0.2.5_x64-setup.exe`（+ `.sig`） | NSIS installer。x64のみ |
| Linux GUI | `kukuri_0.2.5_amd64.AppImage`（+ `.sig`）、`kukuri_0.2.5_amd64.deb`（+ `.sig`） | x86_64 / amd64のみ |
| Linux CLI | `kukuri-cli_0.2.5_x86_64-unknown-linux-gnu.tar.gz`、`kukuri-cli_0.2.5_aarch64-unknown-linux-gnu.tar.gz` | GUIとは別profile |
| 更新 | `latest-preview.json`、`windows-x86_64-updater-key.pub`、`linux-x86_64-updater-key.pub` | 更新manifestは署名あり |
| 検証・表示 | `SHA256SUMS.txt`、`THIRD_PARTY_NOTICES.md`、`release-provenance.json`、`kukuri_0.2.5_linux-native-compliance.json` | |

macOS packageとaarch64 GUI packageは存在しない。素材でmacOS・Android・Web版を示唆しない。

### 素材で使える主張と、その根拠

| # | 主張（LP・告知で使ってよい表現） | 根拠 |
| --- | --- | --- |
| F-1 | 話題（topic）を選んで参加する、話題起点のP2P型SNS | `README.md` 冒頭、`docs/architecture/p2p-first-community-node-responsibility-boundary.md` |
| F-2 | アカウントを識別する鍵は端末内にのみ保存され、中央での再発行・復旧はない | `README.md`「Preview Status and Limits」、`docs/runbooks/mvp-user-quickstart.md`「アカウント鍵だけの持ち出しと復元」 |
| F-3 | 接続の優先順位はDirect P2P → Relay Supported P2P → Relay Fallback | `README.md`「How kukuri Works」、`AGENTS.md`「通信経路」 |
| F-4 | Community Nodeは投稿・プロフィール・社会グラフの恒久的な保管先ではなく、初期接続・認証・rendezvous・接続支援・索引・moderation・通報の範囲で手伝う | `README.md`、`docs/architecture/p2p-first-community-node-responsibility-boundary.md` |
| F-5 | 公開投稿とプライベートチャンネルを同じ話題の中に置ける | `README.md`「What You Can Do」、`apps/desktop/src/components/extended/PrivateChannelPanel.tsx` |
| F-6 | プライベートチャンネルの公開範囲は「招待限定 / Invite only」「相互フォロー限定 / Mutuals」「相互フォロー+ / Mutuals+」の3種類で、既定は招待限定 | `crates/core/src/private_channels.rs:12-20`（`ChannelAudienceKind` の `#[default] InviteOnly`）、`apps/desktop/src/i18n/locales/{ja,en}/channels.json`（`audienceOptions`） |
| F-7 | 日本語・English・简体中文のUIと、light / darkテーマ | `README.md`「Available Today」、`docs/runbooks/mvp-user-quickstart.md` |
| F-8 | 初回起動時に年齢の自己申告とアプリ規約への同意が必要で、同意まではネットワーク接続を開始しない | `README.md`「Try It in 3 Minutes」手順2、`docs/legal/app-consent-data-classification.md` |
| F-9 | Community Nodeごとに、その文書への明示同意が別途必要 | `README.md`、`docs/runbooks/mvp-user-quickstart.md` 手順2 |
| F-10 | Community Indexで投稿と話題を検索・発見できる（提供するNodeに限る） | `README.md`「Available Today」、`apps/desktop/src/i18n/locales/ja/shell.json`（`topicSummary` ほか） |
| F-11 | 相互の関係がある相手とDMでき、画像・動画を添付できる | `README.md`「What You Can Do」 |
| F-12 | アカウントを識別する鍵の暗号化エクスポート／インポートと、端末バックアップ／復元がある | `docs/runbooks/mvp-user-quickstart.md` |
| F-13 | preview installerにOSコード署名はなく、Windows SmartScreenの警告が出うる。アプリ内更新のmanifestは署名され、改ざんされたmanifestは拒否する | `README.md`「Download the Builder Preview」、release notes「Known limits」 |
| F-14 | Builder Preview（テスター向け）であり、一般向けの安定版ではない | `README.md`、release notes |

未解決の主張は0件。次の表現は根拠がないため素材に使わない。

- 「どのサービスとも相互運用できる」（`README.md`「Nostr compatibility is intentionally limited」と矛盾する）
- 「サーバー不要」の断定（Community Nodeへの同意と接続支援が前提の経路がある）
- 「匿名性を保証」（`docs/legal/privacy-policy.md` の範囲を超える）
- 利用者数、導入実績、第三者の推薦、受賞。いずれも根拠がない。

### 既定で使える機能と、開発者モード限定の実験機能

| 区分 | 対象 | 根拠 |
| --- | --- | --- |
| 既定 | トピック発見、Community Index検索、公開投稿、返信・スレッド、リアクション、リポスト、引用、ブックマーク、ミュート・ブロック、プライベートチャンネル（作成・参加・共有）、DM、通知、プロフィール、フォロー、画像・動画添付、鍵のエクスポート／インポート、バックアップ／復元、更新確認、フィードバック送信 | `README.md`「Available Today」 |
| 開発者モード限定（実験中） | Live sessions、Game rooms、Streamカラム、Metaverse Dome一式（Dome hosting、seamless transition、spatial audio、follow camera、connection map、Dome management） | `README.md`「Preview Status and Limits」／「Available Today」の Experimental 行 |

実装上の切り分け（撮影条件の根拠）。

- 開発者モードの状態は `localStorage` の `kukuri.desktop.developer-mode`（`apps/desktop/src/lib/developerMode.ts:1`）。既定は無効。
- 無効時はColumn追加候補から `stream` と `metaverse` を除外する（`apps/desktop/src/shell/page/DesktopShellControlCenter.tsx:156-160`）。
- 無効時は `live` / `game` のroute遷移を受け付けない（`apps/desktop/src/shell/useDesktopShellRouting.ts:164`）。
- 切替は `設定 → 開発者`。

したがって、3場面（S1〜S3）は**開発者モードを無効のまま**撮影する。Dome予告（S9）だけ開発者モードを有効にして別撮りし、manifestの `developerMode` を `true` で記録する。

### 3場面を構成する既定機能の入口

| 場面 | 入口 | 実装上の根拠 |
| --- | --- | --- |
| ①話題を選ぶ | 初期トピック `kukuri:topic:general` / `kukuri:topic:dev` / `kukuri:topic:test`、および「見つける」カラムのCommunity Index検索 | `apps/desktop/src/shell/slices/shared.ts:24-29`、`apps/desktop/src/i18n/locales/ja/shell.json`（`topicSummary`） |
| ②公開で会話する | タイムラインカラムの投稿作成、返信、スレッド、リアクション | `README.md`「Available Today」、`docs/runbooks/mvp-user-quickstart.md` 手順5 |
| ③私的チャンネルへ移る | タイムラインカラム上部の「プライベートチャンネル」ボタン（`workspace.privateChannelEntry`）、またはコントロールセンター → 場所 → チャンネル作成・参加。参加後の共有は「設定と共有」 | `apps/desktop/src/i18n/locales/ja/shell.json:277-278`、`apps/desktop/src/components/extended/PrivateChannelPanel.tsx`、`docs/runbooks/mvp-user-quickstart.md` 手順6 |

### FAQで扱う事実

| ID | 問い | 事実 |
| --- | --- | --- |
| Q-1 | macOS版はありますか | 現時点でpackageを提供していない。Windows 10/11とLinux（x86_64 GUI、x86_64/aarch64 CLI）のみ。 |
| Q-2 | アカウントを失ったら復旧できますか | 中央での再発行・復旧はない。アカウントを識別する鍵を暗号化してエクスポートするか、端末バックアップを自分で保管する。 |
| Q-3 | Community Nodeは必須ですか | 各Nodeの文書へ同意するまで利用しない。「あとで」を選んでも、見つける／設定から後で同意へ戻れる。Nodeは投稿の保管先ではない。 |
| Q-4 | 検索できないのはなぜですか | 接続の手助けと検索（Community Index）の提供は別。検索を提供しないNodeでは、別の検索提供Nodeを選ぶ。 |
| Q-5 | インストール時に警告が出ます | preview installerにOSコード署名がないため。release notesを確認してから実行する。アプリ内更新のmanifestは署名を検証する。 |
| Q-6 | 更新するとデータは残りますか | 鍵、プロフィール、フォロー、自分の投稿、端末内DB、Irohデータ、Community Node設定、プライベートチャンネルの閲覧権限、通知一覧が保持される想定。アンインストール・リセット前にバックアップを取る。 |
| Q-7 | 開発中の機能はありますか | Live、Game、Stream、Metaverse Domeは既定で非表示。`設定 → 開発者` で開発者モードを有効にしたときだけ現れる実験機能で、データ形式の後方互換・移行は保証しない。 |
| Q-8 | 不具合はどこへ報告しますか | アプリ下部バーの「フィードバックを送る」、またはGitHub Issues / Discussions。接続・更新・復旧の問題には、開発者モードで取得した診断レポートを添える。 |

## AC-2: LP原稿（JA/EN）と撮影台本

### 共通の物語

『話題を選ぶ → 公開で会話する → 同じ話題の中で小さな私的チャンネルへ移る』。3場面すべてで同じ架空の話題と同じ2人のデモ参加者を通す。

- 舞台にする話題: `kukuri:topic:dev`（初期トピック。架空のトピックを新規作成せず、既定で存在する話題を使う）
- 会話の題材: 「デスクトップのカラム配置をどう使っているか」という架空のやり取り
- デモ参加者: 表示名 `ふたば（デモ）` / `Futaba (demo)`、`みなと（デモ）` / `Minato (demo)`。どちらも操作者所有のデモidentityであり、実利用者ではない。
- 私的チャンネル名: `カラム配置の相談（デモ）` / `Column layout chat (demo)`、公開範囲は既定の「招待限定 / Invite only」。

すべての画面素材に「デモ画面 / Demo screen」の表記を入れる（表記の置き方はAC-3の共通ルールに従う）。

### LP 7セクションの原稿

#### ① Hero

| 項目 | 日本語 | English |
| --- | --- | --- |
| 見出し | 話題からつながる、あなたの端末が起点のSNS | A topic-first social app that starts on your device |
| 副見出し | 気になる話題を選び、公開で語り、同じ話題の中で小さな輪へ移る。アカウントを識別する鍵は、あなたの端末にだけ置かれます。 | Pick a topic you care about, talk in the open, then move into a smaller circle inside the same topic. The key that identifies your account stays only on your device. |
| CTA（主） | Windows版をダウンロード / Linux版をダウンロード | Download for Windows / Download for Linux |
| CTA（副） | 3分で試す手順を見る | See the 3-minute quickstart |
| 注記 | テスター向けのBuilder Preview（v0.2.5-preview.3）です。一般向けの安定版ではありません。 | This is a Builder Preview for testers (v0.2.5-preview.3), not a stable general release. |

#### ② 3場面

| 場面 | 日本語見出し／説明 | English heading / body |
| --- | --- | --- |
| ①話題を選ぶ | **話題を選ぶ** ／ 最初から用意された話題を開くか、Community Indexで話題と投稿を検索します。検索を提供するのは、あなたが同意したコミュニティノードです。 | **Pick a topic** / Open one of the starter topics, or search topics and posts through the Community Index. Search comes from a Community Node you have consented to. |
| ②公開で会話する | **公開で会話する** ／ 投稿し、返信し、スレッドで続けます。リアクション、リポスト、引用、ブックマークも同じ画面で行えます。 | **Talk in the open** / Post, reply, and keep the thread going. React, repost, quote, and bookmark from the same column. |
| ③私的チャンネルへ移る | **同じ話題の中で、小さな輪へ** ／ タイムライン上部の「プライベートチャンネル」から作成・参加します。話題を離れずに、招待限定・相互フォロー限定・相互フォロー+のいずれかの範囲で話せます。 | **Move into a smaller circle, same topic** / Create or join from "Private channel" at the top of the Timeline column. Stay in the topic and talk within an Invite only, Mutuals, or Mutuals+ audience. |

#### ③ 選べること

| 選べるもの | 初期値 | 変えるとどうなるか（JA） | What changes (EN) |
| --- | --- | --- | --- |
| コミュニティノード | 配布時の候補が1件（同意前は未使用） | 同意したノードだけが接続支援・検索・moderationを提供する。同意しなければ使わない。 | Only a node you consented to assists with connectivity, search, and moderation. Without consent it is not used. |
| プライベートチャンネルの公開範囲 | 招待限定 | 相互フォロー限定、相互フォロー+に変えると、参加できる相手の条件が変わる。 | Switching to Mutuals or Mutuals+ changes who can join. |
| テーマ | dark | lightに変えると、明るい配色に切り替わる。 | Light theme switches the app to the light palette. |
| 表示言語 | OSのUI言語（未保存時） | 日本語 / English / 简体中文を選べる。保存した言語は次回起動でも優先される。 | Choose Japanese, English, or Simplified Chinese. A saved language is kept on the next launch. |
| 成人向け表示 | 非表示 | `設定 → 安全` で許可すると、成人向けラベル付きの投稿が表示される。 | Allowing it in `Settings -> Safety` reveals adult-labeled posts. |
| 開発者モード | 無効 | `設定 → 開発者` で有効にすると、実験中の機能と診断レポートが現れる。 | Enabling it in `Settings -> Developer` reveals experimental surfaces and the diagnostic report. |

#### ④ 分散型である理由

| 項目 | 日本語 | English |
| --- | --- | --- |
| 本文1 | アカウントを識別する鍵は端末内にだけ保存されます。コミュニティノードはあなたのアカウントの持ち主でも、ホームサーバーでもありません。 | The key that identifies your account is stored locally. A Community Node is not your account owner or home server. |
| 本文2 | 接続はDirect P2Pを優先し、次にRelay Supported P2P、実データがrelayを経由するRelay Fallbackは他の経路が成立しないときだけ使います。 | Connectivity prefers direct P2P, then relay-supported P2P, and uses relay fallback only when the earlier paths cannot carry the data. |
| 本文3 | moderationの判定は、それを出したノードからの任意の入力であり、ネットワーク全体への命令ではありません。受け取り方は各クライアントが決めます。 | A moderation event is optional trust input from its issuing node, not a command applied to the whole network. Each client decides how to use it. |
| リンク | 責務境界の詳細（P2P-first Community Node responsibilities） | Read the responsibility boundary |

#### ⑤ Previewの機能・OS・版・手順

| 項目 | 日本語 | English |
| --- | --- | --- |
| 見出し | いま試せること | What you can try today |
| 対応OS | Windows 10 / 11（NSIS）、Linux x86_64（AppImage / deb）、Linux CLI x86_64・aarch64。macOS版はありません。 | Windows 10/11 (NSIS), Linux x86_64 (AppImage / deb), Linux CLI x86_64 and aarch64. There is no macOS package. |
| 版 | v0.2.5-preview.3 | v0.2.5-preview.3 |
| 手順 | 1. 自分の環境向けのインストーラーを取得して起動する 2. 言語を選び、18歳以上であることを申告し、アプリ規約に同意する 3. コミュニティノードの文書を確認して同意する 4. プロフィールを設定する 5. 話題を開いて投稿・返信する 6. 同じ話題の中でプライベートチャンネルを作る・参加する | 1. Download and run the installer for your platform 2. Pick a language, confirm you are 18 or older, and accept the app terms 3. Review and accept the Community Node documents 4. Set up your profile 5. Open a topic, post, and reply 6. Create or join a private channel inside the same topic |
| 注記 | 同意するまでネットワーク接続は始まりません。preview installerにOSのコード署名はなく、Windowsでは警告が出ることがあります。 | No network connection starts until you accept. Preview installers are unsigned, so Windows may show a warning. |

#### ⑥ FAQ・開発状況・feedback・運営／規約

AC-1のQ-1〜Q-8をFAQ項目として使う。並び順はQ-1、Q-2、Q-3、Q-4、Q-5、Q-6、Q-8、Q-7（開発中の機能=Dome予告を最後に置く）。

開発中の実験機能（Dome）の予告文。掲出はこの1件に限る。

| 項目 | 日本語 | English |
| --- | --- | --- |
| 見出し | 開発中の実験機能（開発者モードで有効化） | Experimental, in development (enable developer mode) |
| 本文 | 3D空間で話題に集まるMetaverse Domeを試作しています。`設定 → 開発者` で開発者モードを有効にしたときだけ現れます。 | We are prototyping Metaverse Domes, a 3D space tied to a topic. It appears only after you enable developer mode in `Settings -> Developer`. |
| 必須の表記 | 実験中の機能です。開発者モードでのみ利用でき、データ形式の後方互換や移行は保証しません。 | Experimental. Available only in developer mode, with no backward-compatible decoding or migration guarantee for its data formats. |

feedbackと運営・規約のリンクはAC-4の表に従う。

#### ⑦ 再CTA

| 項目 | 日本語 | English |
| --- | --- | --- |
| 見出し | 話題から始めてみる | Start from a topic |
| 本文 | Builder Previewはテスター向けです。うまくいかない点は、アプリ内のフィードバックかGitHubへ送ってください。 | The Builder Preview is for testers. Tell us what breaks through in-app feedback or GitHub. |
| CTA | Windows版をダウンロード / Linux版をダウンロード / GitHubで報告する | Download for Windows / Download for Linux / Report on GitHub |

### 撮影台本（shot list）

共通条件。特記のない限り、テーマはdark、言語はJA（EN版は同じ操作をEN UIで撮る）、開発者モードは無効、viewportは1600×1000（撮影解像度は#1038の環境契約に従う）。

| scene | cut | 目的 | 操作 | 画角 | 原素材 | 由来 | 字幕（JA / EN） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| S0 | S0-C1 | Hero用の全景 | 3カラム表示で静止 | ワークスペース全体 | 静止画 | mock | なし（Heroはコピーで説明） |
| S1 | S1-C1 | 話題の一覧から選ぶ | タイムラインカラムで話題を切り替える | タイムラインカラム中心 | 動画 | mock | 話題を選ぶ / Pick a topic |
| S1 | S1-C2 | Community Index検索 | 「見つける」で語句を入力し結果を表示 | 見つけるカラム | 動画 | mock | 同意したノードで検索する / Search through a node you consented to |
| S2 | S2-C1 | 公開投稿 | 投稿を作成して送信 | タイムラインカラム＋投稿欄 | 動画 | mock | 公開で投稿する / Post in the open |
| S2 | S2-C2 | 返信とスレッド | 投稿を開いて返信する | スレッドカラム | 動画 | mock | スレッドで続ける / Keep the thread going |
| S2 | S2-C3 | リアクション | 投稿にリアクションする | 投稿カード拡大 | 動画 | mock | 反応を返す / React |
| S3 | S3-C1 | 私的チャンネルの入口 | タイムライン上部の「プライベートチャンネル」を押す | カラム上部を含む | 動画 | mock | 同じ話題の中に、小さな輪 / A smaller circle, same topic |
| S3 | S3-C2 | 作成と公開範囲 | 名前と公開範囲を選んで作成 | チャンネル作成欄 | 動画 | mock | 公開範囲を選ぶ / Choose the audience |
| S3 | S3-C3 | 参加後の会話 | チャンネル内で投稿・返信 | チャンネルのタイムライン | 動画 | mock | 話題を離れずに話す / Stay in the topic |
| S4 | S4-C1 | 2台での実同期 | Windows実機で投稿し、Linux実機に届く | 2画面を並べる | 動画 | 実機（Windows 11 NSIS + Linux AppImage/deb） | 2台の実機で同期 / Synced across two real machines |
| S4 | S4-C2 | 私的チャンネルの実同期 | 招待リンクで参加し、投稿が届く | 2画面を並べる | 動画 | 実機 | 招待して、同じ輪へ / Invite, and join the same circle |
| S9 | S9-C1 | Dome予告 | 開発者モードを有効にし、Domeを数秒映す | Metaverseカラム | 動画（数秒）または静止画1枚 | mock または実機 | 開発中の実験機能。開発者モードでのみ利用でき、データ形式の互換は保証しません / Experimental, developer mode only. No data-format compatibility guarantee. |

- S1〜S3は#1039（mock撮影）が所有する。
- S4は#1040（実機撮影）が所有する。
- S9は#1039または#1040が撮り、どちらで撮ったかをmanifestの `sourceMode` に記録する。
- 実機素材（S4）とmock素材（S1〜S3）を1つのカットの中で混在させない。動画内で切り替える場合は、実機由来のカットに「実機 / Real devices」の表記を出す。

## AC-3: 固定出力一覧とscene IDの対応

### 共通ルール

- デモ表記: 画面を含むすべての出力に「デモ画面 / Demo screen」を入れる。静止画は右下、動画は冒頭2秒と末尾に表示する。人物名には表示名の時点で `（デモ）` / `(demo)` を含める。
- テーマ: アプリ画面はdarkで統一する（`DESIGN.md` §11.1の現行token。背景 `#121212`、パネル `#292929`、アクセント `#03dac5`、primaryボタン `#d77d45`）。LPの地の面は明るいニュートラル＋オレンジとし、デスクトップの高密度レイアウトをLPへ持ち込まない。
- 文字セーフエリア: 静止画は各辺6%、動画は各辺8%を文字の外側余白として空ける。Product Huntのgalleryは上下に各10%を空け、1枚目の見出しは上から18%〜38%の帯に置く。
- 字幕: 動画は無音で理解できることを必須とし、日本語版・英語版のどちらも焼き込み字幕を持つ。
- release表示: 画面内にバージョンが写る場合は `v0.2.5-preview.3` と一致させる。

### 固定出力一覧

| 出力ID | 媒体 | 形式・寸法 | 使用scene | 所有Issue | Dome |
| --- | --- | --- | --- | --- | --- |
| `lp-hero-ja` / `lp-hero-en` | LP | PNG 2400×1500 | S0-C1 | #1041 | 不可 |
| `lp-scene1-ja` / `lp-scene1-en` | LP | PNG 1600×1000 | S1-C1 | #1041 | 不可 |
| `lp-scene2-ja` / `lp-scene2-en` | LP | PNG 1600×1000 | S2-C2 | #1041 | 不可 |
| `lp-scene3-ja` / `lp-scene3-en` | LP | PNG 1600×1000 | S3-C2 | #1041 | 不可 |
| `lp-loop-ja` / `lp-loop-en` | LP | MP4 H.264/yuv420p 30fps 1600×1000 15〜20秒 | S1-C1, S2-C1, S3-C1 | #1042 | 不可 |
| `lp-loop-poster-ja` / `lp-loop-poster-en` | LP | PNG 1600×1000（動画停止時のfallback） | S1-C1 | #1041 | 不可 |
| `lp-dome-teaser` | LP ⑥内 | MP4 3〜5秒 または PNG 1600×1000 | S9-C1 | #1042（動画）／#1041（静止画） | 掲出可（表記必須） |
| `ogp-ja` / `ogp-en` | LP | PNG 1200×630 | S0-C1 | #1041 | 不可 |
| `ph-gallery-1` | Product Hunt (EN) | PNG 1270×760（価値） | S0-C1 | #1041 | 不可 |
| `ph-gallery-2` | Product Hunt (EN) | PNG 1270×760（話題） | S1-C1 | #1041 | 不可 |
| `ph-gallery-3` | Product Hunt (EN) | PNG 1270×760（会話） | S2-C2 | #1041 | 不可 |
| `ph-gallery-4` | Product Hunt (EN) | PNG 1270×760（私的チャンネル） | S3-C2 | #1041 | 不可 |
| `ph-icon` | Product Hunt (EN) | PNG 240×240 | なし（アイコン） | #1041 | 不可 |
| `ph-video` | Product Hunt (EN) | MP4 1920×1080 30fps 30〜60秒 | S1〜S3（+ 末尾にS9-C1可） | #1042 | 末尾のCTA直前のみ可 |
| `note-header` | note (JA) | PNG 1280×670 | S0-C1 | #1041 | 不可 |
| `note-body-1` / `-2` / `-3` | note (JA) | PNG 1280×720 | S1-C1 / S2-C2 / S3-C2 | #1041 | 不可 |
| `x-card-ja` | X (JA) | PNG 1600×900 | S0-C1 | #1041 | 不可 |
| `x-video-ja` | X (JA) | MP4 1600×900 30fps 20〜40秒 | S1〜S3 | #1042 | 末尾のCTA直前のみ可 |

### Domeを含めてはならない出力

次はいかなる形でもDomeの画面・名称・3D空間を含めない。

- `lp-hero-ja` / `lp-hero-en`
- `lp-scene1` / `lp-scene2` / `lp-scene3`（JA/EN）
- `lp-loop` とその `lp-loop-poster`（JA/EN）
- `ogp-ja` / `ogp-en`
- `ph-gallery-1`（Product Huntの1枚目）
- `ph-icon`
- `note-header` / `note-body-1..3`
- `x-card-ja`
- すべての動画の冒頭（`ph-video` と `x-video-ja` の先頭5秒）

Domeを掲出してよいのは `lp-dome-teaser`、および `ph-video` / `x-video-ja` の末尾CTA直前の数秒に限る。いずれの掲出にも、AC-2の「必須の表記」を字幕またはキャプションとして必ず添える。

## AC-4: 公開先と導線

### 現在の公開先（2026-09-18に確認）

| 対象 | 状態 | 確認方法 |
| --- | --- | --- |
| `kukuri.app`（apex） | A / AAAAレコードなし。`https://kukuri.app/` は到達不可 | `nslookup -type=A/-type=AAAA kukuri.app 8.8.8.8`、`curl -o /dev/null -w %{http_code}` → `000` |
| `www.kukuri.app` | NXDOMAIN | `nslookup www.kukuri.app` |
| 権威DNS | Cloudflare（`nova.ns.cloudflare.com` / `keanu.ns.cloudflare.com`） | `nslookup -type=NS kukuri.app 8.8.8.8` |
| `api.kukuri.app` | 稼働中（`35.190.227.178`）。Community NodeのAPIと法務文書を配信 | `nslookup`、各URLへのHTTPステータス |
| GitHub Release | 稼働中。配布物の唯一の公開元 | `gh release view` |

LPを置く `kukuri.app` のapexは現在なにも配信していないため、新LPの公開で既存の公開物を上書きしない。

### 新LPの配置とpreview／本番切替案

ユーザー確定事項（2026-09-18）: ドメインは `kukuri.app`、ホスティングはCloudflare Pages。

1. Cloudflare Pagesのプロジェクトを作り、リポジトリの `apps/lp`（#1043が実装先を確定する）からビルドする。
2. preview環境はPagesが発行するpreviewサブドメインを使い、`kukuri.app` へは接続しない。#1043・#1044の確認はpreview URLで行う。
3. 本番切替は、Cloudflare DNSで `kukuri.app` のapexをPagesプロジェクトへ向ける操作だけで行う。`www.kukuri.app` を作る場合はapexへリダイレクトする。
4. 切替はこのIssue群の完了条件に含めない（#1036の完成範囲はLP previewまで）。切替手順は#1044の公開作業引継ぎに記載し、実行はユーザーが判断する。
5. `api.kukuri.app` の設定は変更しない。LPは同ホストの法務文書へリンクするだけとする。

正規URL（本番切替後）: `https://kukuri.app/`（日本語）、`https://kukuri.app/en/`（英語）。言語切替は相互リンクを持ち、OGPは言語ごとに `ogp-ja` / `ogp-en` を割り当てる。

### LPから張るリンク

| 用途 | URL | 状態 |
| --- | --- | --- |
| ダウンロード（全体） | `https://github.com/kukuri-app/kukuri/releases/latest` | 稼働中 |
| Windows | 上記releaseの `kukuri_0.2.5_x64-setup.exe` | 稼働中 |
| Linux AppImage / deb | 上記releaseの `kukuri_0.2.5_amd64.AppImage` / `kukuri_0.2.5_amd64.deb` | 稼働中 |
| Linux CLI | 上記releaseの `kukuri-cli_0.2.5_*.tar.gz` | 稼働中 |
| quickstart | `https://github.com/kukuri-app/kukuri/blob/main/docs/runbooks/mvp-user-quickstart.md` | 稼働中 |
| troubleshooting | `https://github.com/kukuri-app/kukuri/blob/main/docs/runbooks/mvp-troubleshooting.md` | 稼働中 |
| 変更履歴 | `https://github.com/kukuri-app/kukuri/blob/main/CHANGELOG.md` | 稼働中 |
| feedback（不具合） | `https://github.com/kukuri-app/kukuri/issues` | 稼働中 |
| feedback（提案・質問） | `https://github.com/kukuri-app/kukuri/discussions` | 稼働中 |
| 利用規約 | `https://api.kukuri.app/terms` | HTTP 200 |
| プライバシー | `https://api.kukuri.app/privacy` | HTTP 200 |
| 外部送信 | `https://api.kukuri.app/external-transmission` | HTTP 200 |
| 通報方針 | `https://api.kukuri.app/abuse-policy` | HTTP 200 |
| データ保持 | `https://api.kukuri.app/data-retention` | HTTP 200 |
| 責務境界の説明 | `https://github.com/kukuri-app/kukuri/blob/main/docs/architecture/p2p-first-community-node-responsibility-boundary.md` | 稼働中 |

mobileからの閲覧では、ダウンロードの代わりに「PCで開くためのリンクを控える」導線を出す（実装は#1043）。

### 計測の計画（導入はしない）

#1036のNon-goalsに新規analyticsの導入が入っているため、本Issueでは計画の記載までとする。区別する3つ。

| 指標 | 取り方の案 | 備考 |
| --- | --- | --- |
| 訪問 | Cloudflare Web Analytics（Pagesに同梱、cookieなし・個人識別なし） | 導入可否は公開作業時にユーザーが判断する |
| DLクリック | LP内のダウンロードボタンのクリックを、OS別・言語別に区別して数える | 個人を識別する値を送らない |
| インストール | GitHub Releaseのasset別ダウンロード数（`gh api` で取得可能）を、LPのDLクリックとは別系列として見る | asset DL数はインストール完了数ではないため、近似として扱う |

3指標を1つの数字にまとめない。プライバシーポリシーに記載のない収集を追加しない。

## 維持する契約の確認

| ID | 内容 | 本書での担保 |
| --- | --- | --- |
| INVAR-1 | 未実装機能、架空の利用者数・推薦、デモを実績に見せる表現を含めない | AC-1「素材で使える主張と、その根拠」で使用可能な主張をF-1〜F-14に限定し、使わない表現を明示。デモ表記をAC-3の共通ルールで必須化 |
| INVAR-2 | 既存の製品仕様と公開先を変更せず、#602・#603・#604は過去のClosed Issueとして参照する | 本書は文書のみを追加し、製品コード・DNS・既存公開物を変更しない。`api.kukuri.app` は参照のみ |
| INVAR-3 | 開発者モード限定の実験機能を既定機能として描かず、掲出は末尾予告1件に限り、常に実験中・開発者モード限定の表記を伴う | AC-2でDome予告文と必須表記を固定し、AC-3で掲出可能な出力を `lp-dome-teaser` と2本の動画末尾に限定、非掲出出力を一覧化 |

## 未確認・後続への引き継ぎ

1. 撮影対象releaseは撮影直前に再照合する。`v0.2.5-preview.3` より新しいpreviewが出た場合、#1039 / #1040の着手時に再固定し、本書のScope revisionを更新する。
2. LPの実装先path（`apps/lp` など）とビルド方法は#1043が確定する。本書はホスティング（Cloudflare Pages）と正規URLだけを固定する。
3. Cloudflare Pagesプロジェクトの作成とDNS切替はユーザーの操作であり、#1044の引継ぎ文書に手順を書くまでが本Issue群の範囲。
4. 実機撮影（S4）に使うLinux実機の接続手段は#1040の着手時に確認する。本書では実機素材の由来表記の条件だけを固定した。
5. Remotionのライセンス確認は#1038が所有する。ユーザーの申告では開発者は1名でありFreeライセンスの条件内だが、適用条件と出典の記録は#1038で行う。
