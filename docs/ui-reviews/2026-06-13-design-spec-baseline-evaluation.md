# 2026-06-13 design spec baseline evaluation

- PR: issue [#308](https://github.com/KingYoSun/kukuri/issues/308) のための turn（DESIGN.md をビジュアル仕様へ整備）。
- Figma: N/A。本 turn で Figma レビューフローは破棄した（#308・[ADR 0014](../adr/0014-uiux-dev-flow.md) 参照）。
- Summary: 新しい root [`DESIGN.md`](../../DESIGN.md)（ビジュアル仕様の正本）を整備する前提として、現行 desktop UI のビジュアルシステムをベースライン評価した。**本記録は評価のみ**で、コードの是正は行わない。是正は別 Issue で扱い、各ギャップは `DESIGN.md` の該当セクションに「理想（target）」として定義済み。

## 評価対象

- ランタイムの真実: `apps/desktop/src/styles/tokens.css`（全カラー / radius / shadow トークン）。
- 関連スタイル: `apps/desktop/src/styles/base.css`、`apps/desktop/src/styles/shell-phase1.css`、`apps/desktop/src/components/ui/*`。
- token ショーケース: `apps/desktop/src/stories/foundations/Tokens.stories.tsx`。

## 強み（現状の良い点）

1. **一貫した二色アイデンティティ** — warm-orange の primary/CTA（dark `#f59d62` / light `#d77d45`）と cool-teal の accent（dark `#00b3a4` / light `#0f8c82`）。dark は deep-navy 背景（`#101923`）、light は warm-beige（`#f4efe6`）、本文は warm off-white（`#f6f1e8`）。
2. **dark/light の完全二テーマ** — すべてのセマンティックカラーが `tokens.css` の CSS 変数で `:root[data-theme='light']` 切替に対応。`prefers-color-scheme` ではなく `data-theme` 属性 + localStorage `kukuri.desktop.theme` で正規化。
3. **shadcn primitives + tone システム** — `components/ui/*`（button/card/input/textarea/badge/notice/dialog/popover/tooltip/select/field 等）が CVA + `cn()` + CSS 変数で構築され、Badge/Notice は neutral/accent/warning/destructive の tone を共有。
4. **4 段サーフェス階層** — `--surface-panel` → `-accent` → `-muted` → `-soft` の段階で奥行きとグルーピングを表現。
5. **製品 UI と diagnostics の分離方針が既に言語化済み**（[ADR 0014](../adr/0014-uiux-dev-flow.md) / 旧 `docs/DESIGN.md`）。Tokens story や 29 本の Storybook story も整備されている。

## 検出した 8 ギャップ（file:line 根拠つき）

> 各ギャップ末尾の「→」は、root `DESIGN.md` 側で定義した理想（target）の該当セクション。

1. **未定義トークンの参照バグ** — `var(--warning)` と `var(--danger)` が `apps/desktop/src/styles/shell-phase1.css:322, 326, 637, 640` で使われているが、`tokens.css` に定義が無く描画時に解決されない。存在するのは `--surface-warning-soft`（`tokens.css:36`）/`--border-warning`（`:44`）/`--destructive`（`:51`）/`--surface-destructive-soft`（`:37`）/`--border-destructive`（`:45`）/`--surface-info-soft`（`:38`）のみ。→ `DESIGN.md` §2（セマンティックカラーの三点セット定義）。
2. **日本語フォントのフォールバック不在** — `index.html` は `lang="ja"` だが、`tokens.css:2` の `--font-sans` は `"IBM Plex Sans","Segoe UI",sans-serif` のみで、和文グリフは OS 既定に無制御で落ちる。明朝・縦書きは用途上不要。→ `DESIGN.md` §3（gothic フォールバック追加）。
3. **型階層（type scale）の不在** — 見出し/本文/メタのサイズが `Tokens.stories.tsx:34` の `text-3xl ... tracking-[-0.03em]` などインラインや `clamp()` で散在し、共有トークンも命名スケールも無い。→ `DESIGN.md` §3（型階層テーブル + `--text-*` 提案）。
4. **spacing スケールの不在** — `0.35`/`0.65`/`0.8`/`0.9`/`1rem` 等の rem 値が `shell-phase1.css` 全体に散在。Tailwind デフォルト（4px ベース）を暗黙利用するのみで、命名スケールが無い。→ `DESIGN.md` §5（spacing スケール定義）。
5. **breakpoint の不整合** — `base.css:127` は `@media (max-width:720px)`、`shell-phase1.css` は `@media (max-width:900px)`、JS の `isMobileViewport()` は `<=759` と三者三様。→ `DESIGN.md` §8（breakpoint テーブル / 759 標準への統一案）。
6. **hex 直書き・トークン迂回** —
   - metaverse シーン背景 `#101318`: `shell-phase1.css:253, 266`。
   - dropdown 級の影 `0 12px 32px rgba(2,7,15,0.12)` が直書き: `shell-phase1.css:1543`、`components/ui/notice.tsx:8`、`components/settings/SettingsMetricGrid.tsx:22`。近似値 `rgba(2,7,15,0.1)` も `SettingsDiagnosticList.tsx:29`・`ConnectivityPanel.tsx:88`・`CommunityNodePanel.tsx:95` に散在（＝ギャップ8 と同根）。
   - textarea 高さ `88px`/`120px`: `shell-phase1.css:122, 126`。
   - draft サムネイル幅 `96px`: `shell-phase1.css:94`。
   - `backdrop-filter: blur(14px)` の繰り返し: `shell-phase1.css:281, 313, 347, 522, 2386, 2628`。
   → `DESIGN.md` §7（Do/Don't: hex 直書き禁止・token 化）。
7. **monospace トークンの不在** — `tokens.css` に `--font-mono` が無い。pubkey / event-id / ticket / peer-id を多用する P2P アプリで、ID・ハッシュの可読性が確保されていない（`2026-04-15-...md` でも custom reaction の hash 表示除去に言及）。→ `DESIGN.md` §3（`--font-mono` + tabular-nums）。
8. **elevation スケールが 2 段のみ** — `tokens.css:53-54` の `--shadow-panel` / `--shadow-button-primary` だけで、dropdown / modal / overlay 用の tier が無い。これがギャップ6 の `0 12px 32px` 影の直書き散在を招いている。→ `DESIGN.md` §6（elevation スケール定義）。

## Review result

- 現行のビジュアルシステムは、二色アイデンティティ・二テーマ token・shadcn primitives + tone・製品/diagnostics 分離という強い土台を持つ。
- 一方で、**意味色の欠落バグ（1）**、**和文フォント（2）**、**型/余白/breakpoint の体系化（3-5）**、**hex 直書きと elevation/mono トークンの不足（6-8）** が、仕様の正本化を阻む構造的ギャップとして残る。
- これらは root `DESIGN.md` に「理想（target）」として定義済み。現行 `tokens.css` の実値との差分は、後続の UI 実装 Issue [#325](https://github.com/KingYoSun/kukuri/issues/325) のスコープ。

## Exceptions

- Figma HTML capture artifact: 廃止（#308 でフロー破棄）。
- 本記録は UI behavior/layout 変更ではなく、新 design rule（`DESIGN.md`）整備に伴うベースライン評価。`docs/ui-reviews/README.md` の record 追加条件のうち「reusable design rule を追加/変更する」に該当する。

## Validation

- ドキュメントのみの変更（`apps/desktop/**` のコードは未変更）。`DESIGN.md` の数値表は `apps/desktop/src/styles/tokens.css` と 1:1 照合し、現行値の行は完全一致、それ以外は「将来定義」マーク付き。
- スクリーンショット等の preview は本 turn では未添付（任意）。
