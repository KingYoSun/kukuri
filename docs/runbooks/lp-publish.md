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

ダウンロードのリンクと本文の版表記は `v0.2.5-preview.3` に固定している。新しい release を LP に載せるときは、画面の撮り直しの要否を brief で確認したうえで、`index.html` と `en/index.html` のリンクと版表記を同じ差分で更新する。
