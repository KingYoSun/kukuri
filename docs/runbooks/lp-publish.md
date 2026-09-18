# LP の公開 runbook

`kukuri.app` の LP（`apps/lp/public`）を Cloudflare Pages で配信する手順。制作仕様の正本は [LP・告知素材の共通brief](../progress/2026-09-15-promo-lp-brief.md)。

LP は依存もビルドも無い静的ファイルで、`apps/lp/public` をそのまま配信する。

| パス | 内容 |
| --- | --- |
| `/` | 日本語 |
| `/en/` | 英語 |
| `/assets/` | CSS・JS・画像（画面の静止画と OGP は `assets/screens/`） |
| `_headers` | Cloudflare Pages のセキュリティヘッダーとキャッシュ |

## 画像を作り直す

画面の静止画と OGP は、撮影済みの原素材から作る。撮影と実機の静止画の取り込みは [告知素材の制作 runbook](promo-production.md) を参照する。

```bash
cd tools/promo && node scripts/lp-assets.mjs
```

## 手元で確認する

```bash
python -m http.server 4180 --bind 127.0.0.1 --directory apps/lp/public
```

`http://127.0.0.1:4180/` と `http://127.0.0.1:4180/en/` を開く。

## Cloudflare Pages へ公開する

Cloudflare の資格情報は公開作業をする人が扱う。

### 初回（wrangler を使う場合）

```bash
npx wrangler login
```

```bash
npx wrangler pages project create kukuri-lp --production-branch main
```

先に preview へ出して確認する。

```bash
npx wrangler pages deploy apps/lp/public --project-name kukuri-lp --branch preview
```

表示された preview URL で、日本語・英語、ダウンロードのリンク、スマートフォン表示を確認してから本番へ出す。

```bash
npx wrangler pages deploy apps/lp/public --project-name kukuri-lp --branch main
```

### ダッシュボードを使う場合

Workers & Pages → Create → Pages → Upload assets で、`apps/lp/public` フォルダーをそのまま上げる。

### `kukuri.app` を向ける

Pages プロジェクトの Custom domains で `kukuri.app` を追加する。`kukuri.app` の DNS は同じ Cloudflare アカウントにあるので、レコードは Pages が追加する。`api.kukuri.app` のレコードは変更しない。

公開後に次を確認する。

- `https://kukuri.app/` と `https://kukuri.app/en/` が開く
- ダウンロードのリンクが GitHub Release の配布物を指している
- `https://api.kukuri.app/terms` など規約のリンクが開く
- OGP（`https://kukuri.app/assets/screens/ogp-ja.png`）が取得できる

## 版を上げるとき

LP が案内する release は `apps/lp/release.json` の 1 か所で管理する。ダウンロードのリンク・配布物の名前・本文の版表記は各 HTML に直接書いてあるので、`release.json` を書き換えてからスクリプトで両言語へ反映する。

1. 新しい release の配布物の名前が `kukuri_<version>_...` / `kukuri-cli_<version>_...` の形のままか確かめる（`gh release view <tag>`）。
2. `apps/lp/release.json` の `tag`・`version`・`commit`・`publishedAt` を更新する。
3. 反映して、検査する。

```bash
node apps/lp/scripts/sync-release.mjs
```

```bash
node apps/lp/scripts/sync-release.mjs --check
```

4. ダウンロードのリンクがすべて 200 を返すことを確かめる。
5. 画面の撮り直しが要るかを brief で確認する。画面に版は写っていないので、UI が変わっていなければ撮り直さない。
6. 再 deploy する（上の「Cloudflare Pages へ公開する」）。

`--check` は、HTML に `release.json` と違う版が 1 つでも残っていれば失敗する。
