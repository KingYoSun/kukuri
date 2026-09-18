# 告知素材の制作 runbook

LP・Product Hunt・note・X 向けの素材を、撮影（Playwright）とレンダリング（Remotion）で再生成する手順。

制作仕様の正本は [LP・告知素材の共通brief](../progress/2026-09-15-promo-lp-brief.md)。本書は実行手順だけを扱う。

## 前提

| 項目 | 値 |
| --- | --- |
| Node | `^20.19.0 \|\| >=22.12.0`（検証時 v22.14.0） |
| pnpm | 10.16.1 |
| Playwright | `@playwright/test` 1.62.1（`apps/desktop` の既存依存をそのまま使う） |
| Remotion | 4.0.526（`tools/promo` の専用依存） |
| 撮影 browser | Playwright 同梱の Chromium（検証時 151.0.7922.34） |

`tools/promo` は root の workspace に含めない独立パッケージで、通常の desktop build・release build には入らない。

### Remotion のライセンス

Remotion は個人および従業員3名以下の営利組織に Free License を許諾し、商用の映像・画像制作に使える。4名以上の組織は Company License の購入が必要になる。

- 出典: [LICENSE.md](https://github.com/remotion-dev/remotion/blob/main/LICENSE.md) の "Free License" → "Eligibility"（an individual / a for-profit organization with up to 3 employees）と "Allowed use cases"
- 出典: [remotion.pro/license](https://www.remotion.pro/license)「Remotion is free to use for individuals and companies up to three people」
- 確認日: 2026-09-18
- 現状: kukuri の開発者は1名であり Free License の条件内。人数が4名以上になった時点で Company License へ切り替える。

## 出力先

すべての中間物と成果物は repository root の `promo-artifacts/` に集約する。`.gitignore` 済みで、git へは commit しない。

```
promo-artifacts/
  captures/<sceneId>/<cutId>/   撮影の原素材
    still.png                   静止画
    video.webm                  録画（VP8, viewport と同解像度）
    manifest.json               原素材の由来（version 1）
    props.json                  Remotion へ渡す props
  renders/                      Remotion の出力（PNG / MP4）
  playwright-output/            Playwright 自身の artifact
```

既存の `apps/desktop/test-results/` と `tests/playwright/__screenshots__/`（視覚回帰 baseline）には書き込まない。

## 1. 依存を入れる

```bash
cd apps/desktop && npx pnpm@10.16.1 install
```

```bash
cd tools/promo && npx pnpm@10.16.1 install
```

撮影用の Chromium が入っていない環境では、先に取得する。

```bash
cd apps/desktop && npx pnpm@10.16.1 exec playwright install chromium
```

初回の `remotion still` / `remotion render` は、Remotion が使う Headless Shell（約113MB）を取得する。これは Playwright の Chromium とは別で、レンダリング側が自動で取りに行く。

## 2. 撮影する

```bash
cd apps/desktop && npx pnpm@10.16.1 exec playwright test --config=playwright.promo.config.ts
```

- 既存の `playwright.config.ts` とは port（4177 / 4176）、build 出力（`dist-promo` / `dist`）、test 選択（`tests/promo` / `tests/playwright`）、artifact 出力先が分かれている。既存の test 選択と視覚回帰 baseline は変更しない。
- `worker=1`、`retries=0`。失敗した撮影を retry で上書きせず、失敗として報告する。
- 撮影のたびに対象 cut のディレクトリを作り直すので、前回の残骸が新しい撮影の成果物に混ざらない。
- ブラウザは毎回使い捨ての context で動く。開発機の既存プロファイルは読み書きしない。

撮影対象の release を manifest に残す場合は、環境変数で渡す。

```bash
cd apps/desktop && KUKURI_PROMO_SOURCE_RELEASE=v0.2.5-preview.3 npx pnpm@10.16.1 exec playwright test --config=playwright.promo.config.ts
```

`KUKURI_PROMO_SOURCE_COMMIT` を指定しない場合は、作業ツリーの `git rev-parse HEAD` を記録する。

### 撮影が途中で失敗したとき

そのまま同じコマンドを再実行する。対象ディレクトリは作り直されるため、古い素材が新しい撮影として残ることはない。`video.webm` が空の場合は撮影が失敗として報告され、manifest は書かれない。

## 3. 編集を確認する（Remotion Studio）

```bash
cd tools/promo && npx pnpm@10.16.1 studio --props=../../promo-artifacts/captures/smoke/c1/props.json
```

## 4. 静止画を出す

```bash
cd tools/promo && npx pnpm@10.16.1 still SceneStill ../../promo-artifacts/renders/<出力名>.png --props=../../promo-artifacts/captures/<sceneId>/<cutId>/props.json
```

## 5. 動画を出す

```bash
cd tools/promo && npx pnpm@10.16.1 render SceneClip ../../promo-artifacts/renders/<出力名>.mp4 --props=../../promo-artifacts/captures/<sceneId>/<cutId>/props.json
```

出力は H.264 / yuv420p / bt709 / 30fps。寸法と長さは props の manifest（`viewport` と `clip`）から決まるので、composition 側に固定値を持たせない。

### 出力を確認する

```bash
cd tools/promo && npx pnpm@10.16.1 exec remotion ffprobe ../../promo-artifacts/renders/<出力名>.mp4
```

`Stream #0:0` の行が `h264`、`yuv420p`、意図した解像度、`30 fps` であること、`Duration` が想定の長さであることを確認する。

## 6. 片付ける

```bash
cd tools/promo && npx pnpm@10.16.1 clean
```

render 出力と Playwright artifact だけを消し、撮り直しに時間のかかる原素材は残す。原素材も消す場合は `--captures` を付ける。

```bash
cd tools/promo && npx pnpm@10.16.1 clean -- --captures
```

## props と manifest の契約

`props.json` は撮影時に自動生成され、次の形を持つ。

| key | 意味 |
| --- | --- |
| `manifest` | 原素材の由来一式（下表） |
| `caption` | 焼き込む字幕。無音で理解できるようにするため、動画では原則入れる |
| `demoBadge` | デモ表記を出すか。既定 `true` |
| `fps` | 出力 fps。既定 30 |

manifest の主な項目。

| key | 意味 |
| --- | --- |
| `sceneId` / `cutId` | brief の shot list と対応する場面・カット |
| `sourceCommit` / `sourceRelease` | 撮影対象の commit と release tag |
| `sourceMode` | `mock`（browser mock 撮影）または `device`（実機撮影） |
| `locale` / `theme` / `developerMode` | 画面の言語・テーマ・開発者モードの状態 |
| `viewport` / `recordSize` | 撮影 viewport と録画解像度。異なる場合は縮小が起きているので失敗として扱う |
| `platform` | 撮影した OS・browser・browser の版 |
| `clip` | 採用する区間（ms） |
| `files` / `checksums` | 出力ファイルの相対パスと SHA-256 |

props は必ず JSON ファイルで渡す（`--props=<path>`）。inline の JSON 文字列で渡す運用はしない（Windows の shell quoting で壊れるため）。

### 欠落・不正な入力は失敗する

次はいずれも exit code 1 で終わり、出力ファイルを作らない。

| 入力 | 結果 |
| --- | --- |
| `--props` を渡さない | `promo props: manifest が無い。--props=<manifest を含む JSON> を渡す` |
| `recordSize` が `viewport` と異なる | `promo manifest: 録画解像度が viewport と一致しない (viewport WxH / record WxH)` |
| `SceneClip` に `video` の無い manifest | `promo: scene <id>/<cut> に video が無い` |
| `clip` の長さが 0 以下 | `promo props: clip の長さが 0 以下` |

## 原素材の保管と復元

- 原素材は `promo-artifacts/captures/` に置き、git へは commit しない。大きな動画を通常の git 履歴へ積まない。
- 各 cut の `manifest.json` に SHA-256 があるので、別の場所へ退避したファイルの同一性を確認できる。
- 期限付きの CI artifact を唯一の保管先にしない。採用した素材は、撮影者が保持する別の保管先（外付けドライブ・オブジェクトストレージなど）へ退避し、`manifest.json` を一緒に保管する。
- 退避先から戻すときは `captures/<sceneId>/<cutId>/` の構造ごと戻し、checksum を照合してから render する。
- 原素材を失った場合は、manifest の `sourceCommit` / `sourceRelease` / `locale` / `theme` / `viewport` を同じにして撮り直す。

## 関連

- 制作仕様の正本: [LP・告知素材の共通brief](../progress/2026-09-15-promo-lp-brief.md)
- 視覚仕様: [DESIGN.md](../../DESIGN.md)
- 開発手順全般: [dev.md](dev.md)
