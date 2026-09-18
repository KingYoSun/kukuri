# #1039 デモデータと Playwright 撮影シナリオで UI 原素材を作る

## 対象

- Issue: #1039（統括 #1036）。リスク区分 B
- Scope revision: 2026-09-15-promo-lp-capture-v2
- 作業基準 commit: `4b751946`（#1038 の merge、#1061 PR3、v0.2.6-preview.1 の準備を含む main。作業開始時の `56a8ba15` から rebase した）
- 撮影対象として記録する release: `v0.2.5-preview.3`（`KUKURI_PROMO_SOURCE_RELEASE` で manifest に残す）
- 台本の正本: [LP・告知素材の共通brief](2026-09-15-promo-lp-brief.md) の shot list
- 手順の正本: [告知素材の制作 runbook](../runbooks/promo-production.md)

## 撮影した場面

brief の shot list と、撮影出力の `sceneId` / `cutId` の対応。すべて JA / EN の 2 言語、dark テーマ、1600×1000、開発者モード無効で撮る。

| brief | 内容 | 出力 | 静止画 | 操作 clip |
| --- | --- | --- | --- | --- |
| S0-C1 | Hero 候補の全景 | `s0-hero/c1` | あり | なし |
| S1-C1 | 話題を選ぶ | `s1-topic/c1` | あり | あり |
| S1-C2 | Community Index で検索する | `s1-topic/c2` | あり | あり |
| S2-C1 | 公開で投稿する | `s2-conversation/c1` | あり | あり |
| S2-C2 | 返信してスレッドで続ける | `s2-conversation/c2` | あり | あり |
| S2-C3 | リアクションを返す | `s2-conversation/c3` | あり | あり |
| S3-C1 | 私的チャンネルの入口 | `s3-private-channel/c1` | あり | あり |
| S3-C2 | 公開範囲を選んで作成する | `s3-private-channel/c2` | あり | あり |
| S3-C3 | 作成したチャンネルで話す | `s3-private-channel/c3` | あり | あり |

browser mock では撮らない shot。`captures/index.json` の `notCapturedByMock` にも理由付きで残す。

| brief | 理由 | 担当 |
| --- | --- | --- |
| S4-C1 | Windows 11 実機で書いた投稿が Linux 実機に届く実同期。mock は実通信を行わないため証拠にならない | #1040 |
| S4-C2 | 招待リンクで私的チャンネルへ参加し、投稿が相手の実機に届く実同期 | #1040 |
| S9-C1 | 開発者モード限定の実験機能（Metaverse Dome）。本 Issue は開発者モードを無効のまま撮るため対象外 | #1040 |

## 実装

| path | 内容 |
| --- | --- |
| `apps/desktop/tests/promo/fixtures/demoStory.ts` | 合成のデモ物語。話題は初期トピック `kukuri:topic:dev`、参加者は `みなと（デモ）` / `ふたば（デモ）`（EN は `Minato (demo)` / `Futaba (demo)`）、会話 3 件とリアクション、他トピックに 1 件ずつ。時刻は 2026-09-15T10:00:00Z 起点で固定 |
| `apps/desktop/tests/promo/fixtures/captureScene.ts` | 1 カットの撮影手順（seed 済みのページを開く → 前準備 → guard → 静止画 → 操作の録画 → manifest）と guard |
| `apps/desktop/tests/promo/scenes.spec.ts` | 上表の 9 カット × 2 言語 |
| `apps/desktop/tests/promo/guards.spec.ts` | guard が不完全な画面を採用しないことの確認 |
| `apps/desktop/tests/promo/writeCaptureIndex.ts` | 撮影後に `captures/index.json` を書く globalTeardown。開発者モードで撮った cut が紛れていたら索引を書かずに失敗する |
| `apps/desktop/tests/promo/promoArtifacts.ts` | 出力先を `<sceneId>/<cutId>/<locale>-<theme>/` に分けた。props へ字幕を渡せるようにした |
| `apps/desktop/src/main.tsx` | mock 経路の中だけで、撮影前に `window.__KUKURI_PROMO_MOCK_SEED__` へ置いた seed を読む |
| `apps/desktop/src/mocks/desktopMockModel.ts` / `api/posts.ts` | 下の「mock の変更」を参照 |

### mock の変更

撮影で見つかった mock の不足を 2 点直した。どちらも既存の browser preview と test の挙動を変えない。

1. **新しく作った投稿の時刻**: mock は `created_at` に連番（1, 2, ...）を入れていたため、撮影中に書いた投稿が「1970/01/01」と表示された。`DesktopMockApiOptions.clockBase` を追加し、`created_at = clockBase + 連番` とした。未指定なら 0 で、従来と同じ値になる。
2. **自分の投稿の名前**: mock は自分が作った投稿に `author_name` を載せていなかったため、投稿者が「不明なユーザー」と表示された。実アプリと同じく `myProfile` の名前を載せるようにした。名前が未設定のとき（既定の browser seed）は項目自体を載せず、変更前と同じ形の投稿になる。

あわせて seed 側で、操作者（ふたば）の公開鍵を mock が「自分」として扱う公開鍵（`'f'.repeat(64)`）に揃えた。揃えないと撮影中に書いた投稿が自分の投稿として解決されない。

`sourceMode` の表記は、Issue 本文の `browser-mock` ではなく #1038 で確定した manifest の `mock` を使う。意味は同じ（browser mock 撮影）で、共有 manifest の契約を変えないためである。

### 撮影で分かった製品側の挙動

台本を変える必要はないが、後続（#1041 / #1042）の構図に関わる事実。

- URL で `topic` を指定して開くと、既定のワークスペースの右端に一時 Column が増える。そのため撮影は既定のワークスペースを開き、先頭の Timeline Column で話題を切り替える。
- 私的チャンネルを作ると、既存の Column の右隣に一時 Column として開く。作成 Dialog を閉じると先頭の Column が active に戻り、URL も公開 topic へ戻る。S3-C3 は空の通知・Messages の Column を閉じてからチャンネルを作り、チャンネルの Column を画面内に入れて撮る。
- 作ったばかりのチャンネルには投稿が無い。S3-C3 は操作者が最初の一言を書く場面にした。

## AC / INVAR の証跡

| ID | 証跡 |
| --- | --- |
| AC-1 | 上表の 9 カット × 2 言語 = 18 カットで、静止画 18 枚と操作 clip 16 本（S0 は静止画のみ）を 1600×1000 の原寸で取得した。S0 / S2-C3 / S3-C3 の静止画と clip の最終 frame を目視し、台本の内容（デモ参加者の会話、リアクション、チャンネル内の投稿）と一致することを確認した |
| AC-2 | 同じ fixture・設定で 3 回撮影し、静止画 18 枚がすべてバイト単位で一致した（下の「再現性」） |
| AC-3 | 各カットの `manifest.json` に `sourceMode`・`sourceCommit`・`sourceRelease`・`sceneId`・`cutId`・`locale`・`theme`・`developerMode`・`viewport`・`platform`・`clip`・SHA-256 がある。`captures/index.json` に全カットの一覧と、実機が必要な shot（S4-C1 / S4-C2）と Dome 予告（S9-C1）を理由付きで記載した |
| AC-4 | 撮影のたびに対象ディレクトリを作り直す（#1038）。guard はフォント・画像・文字・文字化け・画面の安定を確かめ、`guards.spec.ts` の 5 件で、不完全な画面 4 種が失敗し正常な画面が採用されることを確認した |
| INVAR-1 | fixture の入力は `demoStory.ts` の合成データだけで、実在の利用者の投稿・プロフィール・鍵・token を使わない。公開鍵は撮影用の固定値 |
| INVAR-2 | mock の画面は実同期の証拠にしない。実同期の shot は `notCapturedByMock` に明記して #1040 へ渡した。通常テストの fixture（`tests/playwright/*-fixture.ts`）は流用していない |
| INVAR-3 | seed が開発者モードを `false` で明示し、全カットで Metaverse / Stream の Column が無いことを撮影前に確かめる。索引作成時に `developerMode: true` の cut があれば失敗する |

## 検証

### 再現性

静止画は、最初は同じ設定で撮り直しても 19 枚中 1〜3 枚が一致しなかった。差分を画素単位で調べると、1 件はプロフィール Column のアバターとフォロー数ボタンの左端（66 画素）で、非同期の読み込みの途中で撮っていた。もう 1 件は入力欄のカーソルの点滅だった。

- 静止画を撮る前に、150ms 間隔の連続 2 回の撮影が一致するまで待つようにした（3 秒で落ち着かなければ失敗）
- 静止画ではカーソルを隠した

変更後、scenes を 3 回撮影して静止画 18 枚がすべて一致した（1 回目と 2 回目、2 回目と 3 回目の比較でそれぞれ 18 / 18）。

### 本番 bundle への非混入

`VITE_KUKURI_DESKTOP_MOCK` を設定せずに本番と同じ `vite build` を行い、出力を検索した。

| 検索語 | 本番 bundle | 撮影用 bundle（`dist-promo`） |
| --- | --- | --- |
| `__KUKURI_PROMO_MOCK_SEED__` | 0 ファイル | 1 ファイル |
| `clockBase` | 0 ファイル | — |
| mock の seed 文字列（`browser mock peer post`） | 0 ファイル | — |

### 変更 path に対応する validation

`apps/desktop/**` の変更に対応する `cargo xtask desktop-ui-check` を、最新 main（`4b751946`）へ rebase した後の作業ツリーでローカル実行し、全 step が通った。

| step | 結果 |
| --- | --- |
| `pnpm lint` | 通過 |
| `pnpm typecheck` | 通過 |
| `pnpm test`（vitest） | 224 files / 1876 tests 通過 |
| `pnpm storybook:build` | 通過 |
| `pnpm test:e2e:browser` | 363 passed |
| `pnpm test:e2e:visual` | 40 passed |

そのほか。

- `cargo xtask oversized-files`: violation 0
- `guards.spec.ts`: 5 passed
- `scenes.spec.ts`: 18 passed（rebase 後の撮り直しを含む）

#### browser test の不安定な 1 件

検証の途中で `tests/playwright/metaverse-connections.spec.ts` の「connection map keyboard and pointer at 390px」が、フルスイート実行時にだけ落ちることがあった。本 Issue の変更が原因かを、同じ手順で繰り返し切り分けた。

| 条件 | フルスイート 3 回の結果 |
| --- | --- |
| rebase 前のベース `56a8ba15`（本 Issue の変更なし） | 失敗・成功・失敗（いずれも同じテスト） |
| 本 Issue の変更あり | 失敗・成功・失敗 |
| 単独実行（`--repeat-each=6`） | 12 回すべて成功 |

変更の有無で失敗の頻度が変わらないため、既存の不安定なテストと判断した。本 Issue の範囲外として別タスクへ切り出した。

切り分けの途中で、mock の自分の投稿に名前を載せる変更が、名前未設定でも `author_name: null` を明示していたことに気付いた。従来は項目自体が無かったので、名前がある場合だけ載せる形に直し、既定の投稿の形を変更前と同じにした。

## 未確認・後続への引き継ぎ

1. 撮影対象 release は `v0.2.5-preview.3` として記録したが、撮影は main の作業ツリーから build した browser mock で行った。画面が配布候補と一致するかは、#1040 の実機撮影と #1044 の統合確認で照合する。
2. フォントはシステムフォント依存で、manifest の `fonts` は空のまま。媒体別の出力でフォントを揃えるかは #1041 が決める。
3. 操作 clip は実時間で動くので、撮り直すと細部が変わる。採用した clip は `manifest.json` の checksum で特定する。
