# kukuri Visual Design Spec

> kukuri desktop（Tauri + React + Tailwind v4 + shadcn/ui）のビジュアル設計仕様。
> 色・文字・余白・幅・影・レスポンシブの具体値を定義する正本。

---

## この文書の位置づけ

- **これは「具体的なビジュアル仕様」であり、実装指示・プロセス方針ではない。** UI/UX のワークフロー・ガードレール・レビューチェックリスト・例外ポリシーは [`docs/adr/0014-uiux-dev-flow.md`](docs/adr/0014-uiux-dev-flow.md) に置く。
- **ランタイムの真実は [`apps/desktop/src/styles/tokens.css`](apps/desktop/src/styles/tokens.css)。** 本書の数値はそこをミラーする。両者が食い違った場合は `tokens.css` を正とし、本書を更新する。
- **「将来定義」と付いた値は、理想（target）として定義するが現行コードには未実装。** その差分は UI 実装 Issue [#325](https://github.com/KingYoSun/kukuri/issues/325) のスコープであり、#308 では定義のみ行う。現行 UI のベースライン評価は [`docs/ui-reviews/2026-06-13-design-spec-baseline-evaluation.md`](docs/ui-reviews/2026-06-13-design-spec-baseline-evaluation.md) を参照。
- 本書は **dark-first**（dark がデフォルトテーマ、light は opt-in）。アイデンティティは既存の二色（warm-orange × cool-teal）を洗練したものとし、hue ファミリーは変えない。

---

## 1. ビジュアルテーマと雰囲気

- **デザイン方針**: deep-navy の dark シェルを基調に、warm-orange の primary/CTA と cool-teal の accent を効かせた「落ち着いた高密度ワークスペース」。トピック・スレッド・ポスト・ピアといった分散ソーシャルの情報が主役で、装飾は控えめにする。
- **二色アイデンティティ**: warm-orange（暖色）＝行動を促す primary/CTA、cool-teal（寒色）＝accent・focus・選択状態。暖色と寒色のコントラストで「操作できる場所」を明確化する。
- **密度**: timeline / thread / post の閲覧面はやや穏やかな余白、diagnostics（connectivity / discovery / community-node）などの ops 表示は密度を上げすぎず、区切り線と見出しでスキャンしやすくする。
- **キーワード**: 落ち着き、温かみ、分散の信頼感、控えめなアクセント、長時間でも疲れにくい。
- **特徴**:
  - **4 段サーフェス階層** `--surface-panel`（base）→ `-accent` → `-muted` → `-soft` で奥行きとグルーピングを表現する。
  - **大きめの角丸**: パネル 22px、入力 14px、ボタン/チップは pill（999px）。
  - **拡散の弱い影**: 低不透明・大ぼかしの影で、面を浮かせすぎない。
  - **solid 面**: 半透明グラデーションではなく不透明なサーフェスで階層を作る（[ADR 0014 / theme solidification の方針](docs/ui-reviews/) を継承）。
- **kukuri 固有（gestaloka との違い）**: kukuri は **dark-first**（gestaloka は cream / 紙質感の light-first）。物語本文・明朝体・縦書きは扱わない。gestaloka の「閲覧 vs 管理」の密度分けは、kukuri では **製品コンテンツ vs diagnostics** の階層分けに読み替える。

---

## 2. カラーパレットと役割

全色は dark / light の二テーマを 1 セットで定義する（`tokens.css`）。各表は `Dark (default)` と `Light` の二列。**「将来定義」行は現行 `tokens.css` に未定義**。

### 2.1 Brand / Action

| Token | 役割 | Dark | Light |
|-------|------|------|-------|
| `--primary-start` / `--primary-end` | primary/CTA 基色（現状は単色、グラデーション拡張可） | `#f59d62` | `#d77d45` |
| `--surface-button-primary` | primary ボタン面 | `#f59d62` | `#d77d45` |
| `--surface-button-primary-hover` | primary ボタン hover | `#ee8f4e` | `#c86f38` |
| `--primary-foreground` | primary 上の文字色 | `#0e1b26` | `#fff7ef` |
| `--accent` | accent（teal）: 強調・focus・選択 | `#00b3a4` | `#0f8c82` |
| `--accent-foreground` | accent 上の文字色 | `#eafffb` | `#143633` |
| `--surface-accent-soft` | accent の淡面 | `#17393c` | `#d8eee9` |
| `--surface-active` | アクティブ/選択面 | `#17393c` | `#d8eee9` |
| `--surface-selection` | テキスト選択（`::selection`） | `#d98b55` | `#e9b28c` |
| `--surface-button-secondary` | secondary ボタン面 | `#233241` | `#dfe6ec` |
| `--surface-button-ghost` | ghost ボタン面 | `#1a2734` | `#edf2f6` |
| `--surface-button-ghost-hover` | ghost ボタン hover | `#223241` | `#e3ebf1` |

### 2.2 Semantic（意味的な色）

**既存**:

| Token | 役割 | Dark | Light |
|-------|------|------|-------|
| `--destructive` | 破壊的アクションの文字/アイコン | `#ffb48a` | `#b35f46` |
| `--surface-destructive-soft` | destructive 淡面 | `#4a2b22` | `#f6dfd4` |
| `--border-destructive` | destructive 境界 | `#a35e49` | `#d89b86` |
| `--surface-warning-soft` | warning 淡面 | `#463423` | `#f6e7d9` |
| `--border-warning` | warning 境界 | `#a36b40` | `#d1a06d` |
| `--surface-info-soft` | info 淡面 | `#203449` | `#dce7f4` |
| `--warning` | warning の前景（文字/アイコン） | `#e6b066` | `#9a6e2a` |
| `--danger` | danger/error の前景（`--destructive` と統一） | `#ffb48a` | `#b35f46` |

> `--warning` / `--danger` は #325 で定義し、`shell-phase1.css` の未定義参照（旧 評価記録ギャップ1）を解消した。

**将来定義**（既存の `-soft` ファミリーと対になる前景色。現状コードに consumer が無いため未実装。consumer 追加時に定義する）:

| Token | 役割 | Dark | Light |
|-------|------|------|-------|
| `--info` | info の前景（`--surface-info-soft` と対） | `#7fb1e0` | `#2c6aa6` |
| `--success` | success の前景（accent teal 寄り） | `#34c39a` | `#2f8f6e` |
| `--surface-success-soft` | success 淡面 | `#17352c` | `#dff0e6` |
| `--border-success` | success 境界 | `#2f8f6e` | `#8cc2a6` |

### 2.3 Neutral / Surface（4 段サーフェス + 補助面）

| Token | 役割 | Dark | Light |
|-------|------|------|-------|
| `--background` / `--shell-background` | ページ / シェル背景 | `#101923` | `#f4efe6` |
| `--surface-panel` / `--surface-panel-solid` | パネル基面（base） | `#0c1721` | `#ffffff` |
| `--surface-panel-accent` | パネル（accent 段） | `#162231` | `#f5ede2` |
| `--surface-panel-muted` | パネル（muted 段） | `#13202c` | `#edf2f6` |
| `--surface-panel-soft` | パネル（soft 段） | `#182632` | `#e6edf2` |
| `--surface-input` | 入力面 | `#101b26` | `#f8f4ee` |
| `--surface-raised` | 持ち上げ面 | `#1b2a36` | `#dde5ec` |
| `--surface-overlay` | オーバーレイ / backdrop | `#071019` | `#d7dfe7` |
| `--surface-contrast` | コントラスト面 | `#20303c` | `#dde5ec` |
| `--surface-avatar` | アバター背景 | `#21303d` | `#dfe8ee` |
| `--surface-skeleton` | スケルトン | `#243442` | `#e8eef3` |
| `--surface-media-loading` | メディア読込中 | `#1a2734` | `#dde5ec` |
| `--surface-media-ready` | メディア表示 | `#173439` | `#d8eee9` |
| `--surface-badge-neutral` | badge 中立面 | `#1a2734` | `#edf2f6` |

### 2.4 Text（テキスト色）

dark がデフォルトのため、dark 列がそのまま基準値。light 列が opt-in 時の上書き値。

| Token | 役割 | Dark | Light |
|-------|------|------|-------|
| `--foreground` | 本文テキスト | `#f6f1e8` | `#21303b` |
| `--foreground-strong` | 強調本文 / 見出し | `#fff7ef` | `#15202a` |
| `--muted-foreground` | 補助テキスト | `#cbbdae` | `#5f6c76` |
| `--muted-foreground-soft` | さらに淡い / placeholder | `#a89b8f` | `#74818a` |

### 2.5 Border / Focus / Scrollbar

| Token | 役割 | Dark | Light |
|-------|------|------|-------|
| `--border-subtle` | 標準境界 | `#2a3a4a` | `#cad3db` |
| `--border-subtle-strong` | 強い境界 | `#39495a` | `#b7c2cb` |
| `--border-accent` | accent 境界 | `#2d7b76` | `#78a8a2` |
| `--ring` | focus リング（teal） | `rgba(0,179,164,0.45)` | `rgba(15,140,130,0.32)` |
| `--scrollbar-track` | スクロールバー軌道 | `#12202c` | `#edf2f6` |
| `--scrollbar-thumb` | スクロールバー摘み | `#2a4d56` | `#b8c6d2` |
| `--scrollbar-thumb-hover` | 摘み hover | `#38717c` | `#93a8b8` |

---

## 3. タイポグラフィ

### 3.1 フォントスタック

```css
/* sans（既定）: 欧文 → 和文（macOS ヒラギノ → Windows 游ゴシック / Noto / メイリオ）の順 */
--font-sans: "IBM Plex Sans", "Hiragino Kaku Gothic ProN", "Yu Gothic", "Noto Sans JP",
  "Meiryo", "Segoe UI", sans-serif;

/* mono: pubkey / event-id / ticket / peer-id / hash 用 */
--font-mono: "IBM Plex Mono", "Cascadia Code", "Consolas", SFMono-Regular, monospace;
```

- 欧文を先頭に置き欧文の表示品質を優先、続けて和文（macOS ヒラギノ → Windows 游ゴシック / Noto / メイリオ）へフォールバックする。
- **明朝体・縦書きは扱わない**（物語本文のような長文読み物面が無いため N/A）。
- body には `font-feature-settings: "ss01" 1` を適用済み（`base.css`）。`code` / `kbd` / `samp` 要素は `--font-mono` + `font-variant-numeric: tabular-nums` で表示する（`base.css`）。pubkey / ticket / hash を `<code>` で囲むと mono 表示になる。インライン diagnostics（peer id / endpoint 等）への展開は後続。
- 長い URL / pubkey / ticket の折り返しに `overflow-wrap: anywhere` を使う。

### 3.2 型階層

font-size は `--text-*` トークンに集約済み（#325）。kukuri は dark-first の高密度アプリのため、単一スケールに密度の異なる用途を載せる。weight / line-height / letter-spacing は各 role の指針（font-size のようにはトークン化していない）。

| Role | Token | Size | Weight | Line-height | Letter-spacing | 用途 |
|------|-------|------|--------|-------------|----------------|------|
| Display | `--text-display` | clamp(1.9rem, 4vw, 3.5rem)（~30–56px、流動的） | 600 | 0.94–1.2 | -0.03em | トピック見出し / ヒーロー |
| Heading 1 | `--text-h1` | 24px (1.5rem) | 600 | 1.3 | -0.02em | ワークスペース主見出し |
| Heading 2 | `--text-h2` | 20px (1.25rem) | 600 | 1.35 | -0.01em | パネル / セクション見出し |
| Heading 3 | `--text-h3` | 16px (1rem) | 600 | 1.5 | normal | カード見出し |
| Body | `--text-body` | 14px (0.875rem) | 400 | 1.5 | normal | 既定の本文・入力 |
| Body Reading | `--text-body-reading` | 15px (0.9375rem) | 400 | 1.6 | normal | post / thread 本文 |
| Body Strong | `--text-body` | 14px (0.875rem) | 600 | 1.5 | normal | 強調本文（weight 600） |
| Caption / Meta | `--text-caption` | 12px (0.75rem) | 400–600 | 1.5 | normal | メタ情報・補助 |
| Eyebrow / Label | `--text-caption` | 12px (0.75rem) | 600 | 1.4 | 0.08em（uppercase） | ラベル・badge |
| Mono / ID | `--font-mono` | 12–14px | 400 | 1.5 | normal | pubkey / ticket / hash（+ tabular-nums） |

- `letter-spacing` の負値（字詰め）は **Display / Heading にのみ**適用し、本文・入力には適用しない。
- `0.08em` + uppercase は **Eyebrow / Label / Badge にのみ**適用する。

---

## 4. コンポーネントスタイル

実装は [`apps/desktop/src/components/ui/`](apps/desktop/src/components/ui/) と [`apps/desktop/src/styles/shell-phase1.css`](apps/desktop/src/styles/shell-phase1.css)。

### 4.1 Buttons（[`ui/button.tsx`](apps/desktop/src/components/ui/button.tsx) / CVA）

- **形状**: pill（`border-radius: 999px`）。icon ボタンのみ角丸 14px。
- **サイズ**: `default` = `min-h-11`（44px）/ `px-4 py-3`、`sm` = `min-h-9`（36px）/ `px-3 py-2` / `text-sm`、`icon` = `size-10`（40px）。
- **variant**:
  - `primary`: 面 `--surface-button-primary`、hover `--surface-button-primary-hover`、文字 `--primary-foreground`、影 `--shadow-button-primary`。
  - `secondary`: 面 `--surface-button-secondary`、文字 `--foreground`、影なし。
  - `ghost`: 面 `--surface-button-ghost`、hover `--surface-button-ghost-hover`、文字 `--foreground`、影なし。
- `disabled` は `opacity: 0.56`、`cursor: not-allowed`（`base.css`）。

### 4.2 Cards / Panels（`.panel`）

```css
.panel {
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-panel); /* 22px */
  padding: 1rem;
  background: var(--surface-panel-solid);
  box-shadow: var(--shadow-panel);
}
.panel-accent { background: var(--surface-panel-accent); }
```

- diagnostics パネルはカードを入れ子にせず、境界線と見出しで情報を整理する。
- 参考コンポーネント: `PostCard` / `ThreadPanel` / `AuthorDetailCard`。

### 4.3 Input / Textarea（[`ui/input.tsx`](apps/desktop/src/components/ui/input.tsx) / [`ui/textarea.tsx`](apps/desktop/src/components/ui/textarea.tsx)）

- 高さ `h-11`（44px）、角丸 `var(--radius-input)`（14px）、境界 `--border-subtle`、面 `--surface-input`、placeholder `--muted-foreground-soft`。
- focus: `focus-visible:ring-2 ring-[var(--ring)]`。disabled: `opacity-60`。
- textarea の最小高さは現状 88px / 120px の直書き（`shell-phase1.css:122,126`）→ 将来トークン化（評価記録ギャップ6）。

### 4.4 Badge / Notice（tone システム）

- tone: `neutral` / `accent` / `warning` / `destructive`。それぞれ `--surface-*-soft` + `--border-*`（+ 文字色）の組で表現。
- **Badge**: `rounded-full`、`px-2.5 py-1`、`text-xs font-semibold tracking-[0.08em] uppercase`。
- **Notice**: 角丸 `var(--radius-input)`、`px-4 py-3 text-sm leading-6`。影は現状 `0 12px 32px rgba(2,7,15,0.12)` の直書き（`ui/notice.tsx:8`）→ 将来 `--shadow-dropdown` 化（§6・評価記録ギャップ6,8）。

### 4.5 Navigation（[`shell/page/DesktopShellControlCenter.tsx`](apps/desktop/src/shell/page/DesktopShellControlCenter.tsx)）

- 常設 nav rail は置かず、左下の Control Center trigger から Column、場所、アクティビティ、システムへ移動する。
- 現在の Column は一覧とmobile position indicatorの両方で示し、直接jumpとkeyboard操作を提供する。
- グローバル導線は短いラベルと安定配置を優先し、製品コンテンツの邪魔をしない。

### 4.6 入口 / 空・オンボーディング状態

kukuri にはマーケティング的な First View / ブランドロックアップは無い。代わりに **シェル入口**と**空・読込・エラー状態**を整える。

- 中央寄せ状態の参考: `.startup-error-screen`（`base.css`）= `min-height:100vh; display:grid; place-items:center` + `.panel` 風カード。
- オンボーディングは starter topics（`kukuri:topic:demo` 他）を提示する初回体験を前提にする。
- 意味のある面には loading / empty / error / success 状態を必ず定義する（ガードレールは [ADR 0014](docs/adr/0014-uiux-dev-flow.md)）。

### 4.7 スタイリング層とファイル構成（WP-H8）

スタイルは 2 層に分ける。**新規スタイルはこのルールに従って置き場を選ぶ。**

- **プリミティブ層 = `ui/` コンポーネント**（[`ui/button.tsx`](apps/desktop/src/components/ui/button.tsx) 等）: Tailwind ユーティリティ + CVA（`buttonVariants` 等）+ `tokens.css` の CSS 変数で完結させる。**semantic クラス（`.post-card` 等）を持たせない**。variant は CVA に足す。
- **セマンティック層 = `styles/` の shell スタイルシート**: `.post-card` / `.composer` / `.shell-*` などアプリ固有のクラス。core / extended / shell の各コンポーネントが `className` で参照する。**新規の semantic クラスはこの層にのみ足す**（コンポーネント内にインラインの巨大 style を書かない）。

`styles/` の構成（`index.css` がこの順で `@import`。cascade はこの順序が正）:

| ファイル | 役割 |
|---|---|
| `tokens.css` | CSS 変数（色 / spacing / 型 / 角丸）。全層の基盤 |
| `base.css` | リセット + 全体既定 + `.startup-error-screen` 等の非 shell 面 |
| `shell-phase1-part1〜4.css` | shell の semantic クラス本体。WP-H8 PR4 で 1 ファイルを **cascade 順を保つ連続セグメント**に 4 分割した（ドメイン純粋ではない。番号順に `@import` して元の 1 ファイルと同一 cascade を再現する。1 ファイル 1,000 行未満に収める churn 対策） |
| `shell-scoped-overrides.css` | `.shell-phase1` スコープ付き上書き層。shell 配下では `shell-phase1-part*` を specificity で上書きし、body 直下の Radix portal（dialog / tooltip / dropdown）には効かない。**portal と shell で意図的に値が異なるクラスの shell 側の値**をここに置く |

- portal（Radix の Dialog / Tooltip / DropdownMenu / Popover）は `body` 直下に描画され `.shell-phase1` スコープの外にある。同じ semantic クラスを portal と shell で別値にしたい場合のみ、base 値を `shell-phase1-part*` に、shell 上書き値を `shell-scoped-overrides.css` に置く二層構造にする。
- `var(--token)` は同梱スタイルシート内で定義済みのものだけ参照する（`css-vars.test.ts` が未定義参照を検出する）。

### 4.8 shell 状態層とファイル構成（WP-H6 / Q3 / Q4 / B4 / B5、規約化は WP-B10）

`apps/desktop/src/shell/` の状態・データ取得・表示変換は次の層に分ける。**新規コードはこのルールに従って置き場を選ぶ**（4.7 の CSS 層規約と対になる state 層の規約）。

| 置き場 | 役割 |
|---|---|
| `slices/` | store の**状態の形と初期値**（ドメイン別スライス。単一 store の交差型合成。`set()` の原子性維持のため store は分割しない） |
| `storeSelectors.ts` | **複数の hook が共有する**名前付き購読スライス（`useShallow` 前提）。単一コンポーネント専用の購読は、そのコンポーネント内のインライン `useShallow` でよい（2 流儀はこの使い分けが意図） |
| `presentation.ts` | store 非依存の**表示変換 helper**（純関数） |
| `data/` + `data/loaders/` | **取得系 hook**。section 取得ロジックの SSoT は `loaders/`（WP-B5）で、`useDesktopShellDataEffects` の section effect はトリガ判定（+ messages の interval 管理）だけを持つ |
| `viewModels/` | **section 別の projection hook**（timeline / channels / profile / messages / settings。WP-Q3 / B4）。`useDesktopShellViewModels` はその合成 facade で、section に属さない shell 横断 projection（chrome / topic nav / composer / thread / route コピー）だけを直接持つ |
| `actions/` | **API 副作用を持つ action creator**（操作フロー別） |
| `page/` | **画面合成**と page 専用 hook（dialog / focus / share preview） |
| `routing/` | URL ↔ `RouteState` の**同期 adapter**（純関数は `routes.ts` が正） |

- 層ごとの分類軸は意図的に異なる（slices = 状態ドメイン / actions = 操作フロー / viewModels = 表示 section）。層をまたいで名前を無理に揃えない。
- selector なしの全ストア購読（`useDesktopShellStore()` 引数なし）を新規に書かない（WP-H6 で全廃済み。型面では書けてしまうため、防波堤はレビュー）。

---

## 5. レイアウト原則

### 5.1 旧シェル構造（撤去済み）

- 左 nav rail＋main workspace＋右 detail pane stackの3カラム構造と、`ShellFrame` / `ContextPane` / `ShellNavRail`はIssue #748でproduction・Storybookから撤去した。
- Thread / Profileを右paneへ積む旧分岐、mobile footer、`data-detail-pane-count`による予約幅は残さない。
- hash route contractは維持し、route targetは§5.2のColumn stateと同期する。

### 5.2 Column Canvas と可変 span（ADR 0031）

保存済み layout がない初期表示は、中央寄せした Timeline Column 1本、Column 下部の primary action、左下の Control Center trigger だけにする。常設 Sidebar、global workspace tab header、detail pane の予約領域を置かない。

desktop の Column unit は `--column-unit: 27.5rem`（440px）、Column 間 gap は `--column-gap: var(--space-md)`（16px）とする。表示幅は次で求める。

```text
width = span * columnUnit + (span - 1) * gap
```

| Column kind | Desktop default | Desktop width | Mobile |
|---|---:|---:|---:|
| Timeline / Notifications / Profile / Thread | 1 span | 440px | 1 viewport |
| Messages / Conversation | 1〜2 span | 440〜896px | 1 viewport |
| Stream | 2 span | 896px | 1 viewport |
| Metaverse | 3 span | 1352px | 1 viewport |
| Metaverse focused | 最大4 span | 1808px | 1 viewport |

- 複数 span Column は分割不能な atomic surface とし、drag ghost と並べ替えでも元 span を保つ。
- Window が狭い場合は actual width を縮めてよいが、保存した preferred desktop span は失わない。
- internal layout は Column 自身の実幅に基づく container query 等で切り替え、viewport 幅だけに依存しない。
- 横方向の overflow は Column Canvas 内に閉じ込め、document-level の横 overflow を発生させない。

### 5.3 Column state と focus

| State | 視覚表現 | 非視覚表現 |
|---|---|---|
| active | accent border + header indicator | `aria-current` と state label |
| focused | teal focus ring | DOM focus |
| pinned | pin icon + `Pinned` label | `aria-pressed='true'` |
| transient | dashed border + `Temporary` label | state label |
| dragging | elevation + grip state label | `aria-grabbed` 相当の説明 |
| drop target | insertion line + position label | live region / position copy |

- active と focused は別 state とし、partially visible な Column を active とみなさない。
- drag grip は Column header に置いて focus 可能にする。Column 本体、post、media、3D viewport、text selection を grip にしない。
- Column menu に close、pin、span、左へ移動、右へ移動を置き、drag を唯一の並べ替え手段にしない。
- screen reader に Column title、位置、総数、span、active / pinned / transient state を伝える。

### 5.4 Control Center

- desktop trigger の既定位置は左下。左側 / 左下をアプリ全体、各 Column 下部を Column 固有 action とする。
- Control Center は画面下から開く bottom drawer とし、通常の移動では background interaction を完全に遮断しない。
- drawer は Column、場所、アクティビティ、システムの4区分を持つ。設定編集、認証、同意、破壊的確認だけ modal Sheet / Dialog へ遷移する。
- trigger は最小44px、状態 dot / unread badge、accessible name、visible focus を持つ。Community Node 障害は trigger の状態と affected Column の inline Notice の両方で示す。
- mobile では safe area を避け、Column position indicator と下部 action / Composer の操作領域を塞がない。

### 5.5 spacing スケール

`gap` / `padding` / `margin` は `--space-*` トークンに集約済み（#325、4px ベース）。off-grid 値は最寄りの step へ正規化した。

| Token | 値 | px |
|-------|----|----|
| `--space-2xs` | 0.25rem | 4 |
| `--space-xs` | 0.5rem | 8 |
| `--space-sm` | 0.75rem | 12 |
| `--space-md` | 1rem | 16 |
| `--space-lg` | 1.5rem | 24 |
| `--space-xl` | 2rem | 32 |
| `--space-2xl` | 3rem | 48 |

### 5.6 角丸スケール

`border-radius` は以下のトークンに集約済み（#325）。

| Token | 値 | 用途 |
|-------|----|------|
| `--radius-xs` | 0.5rem (8px) | チップ・小コントロール |
| `--radius-sm` | 0.75rem (12px) | 小カード・サムネイル |
| `--radius-input` | 14px | 入力・Notice・小面 |
| `--radius` | 16px (1rem) | 標準（`--radius-md`=16px / `--radius-lg`=22px の基準） |
| `--radius-panel` | 22px | パネル・カード |
| `--radius-pill` | 999px | ボタン・チップ・バッジ |

---

## 6. 奥行きとエレベーション

### 6.1 Shadow

**既存（実装済み）**:

| Token | Dark | Light | 用途 |
|-------|------|-------|------|
| `--shadow-panel` | `0 18px 60px rgba(2,7,15,0.22)` | `0 18px 48px rgba(33,48,59,0.12)` | カード・パネル |
| `--shadow-dropdown` | `0 12px 32px rgba(2,7,15,0.12)` | `0 12px 32px rgba(33,48,59,0.1)` | popover / dropdown / notice / メトリクス |
| `--shadow-button-primary` | `0 10px 28px rgba(245,157,98,0.16)` | `0 10px 24px rgba(215,125,69,0.18)` | primary ボタン |

- `--shadow-dropdown` は #325 で定義し、複数ファイルに直書きされていた `0 12px 32px …` 影を集約した（旧 評価記録ギャップ8）。
- `backdrop-filter` のぼかし量は `--blur-hud`（`14px`、#325）に集約。metaverse 3D ビューポート背景は `--surface-metaverse`（`#101318`）。
- すべて低不透明・大ぼかしの拡散影で統一し、面を浮かせすぎない。

**将来定義**（consumer が無いため未実装。dialog / overlay に専用影を入れる際に定義する）:

| Token | Dark | Light | 用途 |
|-------|------|-------|------|
| `--shadow-modal` | `0 28px 80px rgba(2,7,15,0.30)` | `0 28px 64px rgba(33,48,59,0.16)` | dialog / modal |
| `--shadow-overlay` | `0 0 0 100vmax rgba(7,16,25,0.55)` | `0 0 0 100vmax rgba(33,48,59,0.30)` | backdrop / overlay |

### 6.2 Motion

motion は state 変化と hierarchy を説明する目的に限定し、位置移動、fade、drawer の開閉で同じ token を使う。

| Token | 値 | 用途 |
|---|---:|---|
| `--motion-duration-fast` | 120ms | hover、focus、indicator |
| `--motion-duration-standard` | 200ms | Column state、menu、短い移動 |
| `--motion-duration-slow` | 280ms | Control Center drawer、Column 追加 / 移動 |
| `--motion-easing-standard` | `cubic-bezier(0.2, 0, 0, 1)` | 通常の state transition |
| `--motion-easing-enter` | `cubic-bezier(0, 0, 0, 1)` | surface の出現 |
| `--motion-easing-exit` | `cubic-bezier(0.3, 0, 1, 1)` | surface の退出 |
| `--motion-distance-column` | 24px | Column 追加 / 移動の最大距離 |
| `--motion-distance-control-center` | 32px | Control Center drawer の説明的移動量 |

- `prefers-reduced-motion: reduce` または Storybook の `data-reduced-motion='reduce'` では duration を 1ms、移動距離を 0 にする。
- reduced motion でも state の最終表示、focus、active、drawer open / closed は省略しない。
- viewport 端の drag auto-scroll、scroll snap の補間、fullscreen 復帰で長い animation を強制しない。
- opacity だけで active / pinned / transient / dragging / drop target を区別しない。

---

## 7. アプリケーションルール（Do / Don't）

> ここに置くのは **視覚的なルール**のみ。ワークフロー / レビュー成果物 / Shneiderman チェックリスト / 検証ゲート / 例外ポリシーは [ADR 0014](docs/adr/0014-uiux-dev-flow.md)。

### Do（推奨）

- 色・余白・radius・影は `tokens.css` のトークンから取る。
- 製品 UI（timeline / thread / post / channel）と diagnostics UI（connectivity / discovery / community-node）を**視覚階層で分離**し、diagnostics を後景に置く。
- warm-orange は primary action（CTA）に限定し、cool-teal は accent / focus / 選択に限定する。
- 4 段サーフェス（base → accent → muted → soft）で階層を表現する。
- 意味のある面に loading / empty / error / success 状態を定義する。
- pubkey / ticket / hash は `--font-mono` + tabular-nums で表示する（将来定義）。
- focus リング（`--ring`）を常に視認できる状態に保つ。

### Don't（禁止）

- hex を直書きしない（例: metaverse `#101318`、影 `0 12px 32px rgba(...)`）。トークン化する。
- `--shadow-panel` / `--shadow-dropdown` を無視した独自影を作らない。
- warm-orange を装飾目的で乱用しない（行動喚起の意味が薄れる）。
- cool-teal を danger / error と取り違える配色をしない。
- 和文テキストのフォントフォールバックを未指定のまま放置しない（`lang="ja"`）。
- 半透明グラデーションで階層を作らない（solid 面 + 境界 + 影で表現する）。

---

## 8. レスポンシブ挙動

### 8.1 テーマ機構（dark-first）

- `<html data-theme="dark|light">` 属性で切替。デフォルトは `dark`。
- 永続化は localStorage key `kukuri.desktop.theme`（`lib/theme.ts`）。
- **`prefers-color-scheme` は使わない**（OS 設定に追従しない）。
- native control の描画 hint は app theme と一致する `color-scheme: dark|light` を `tokens.css` から与える。`select` / `option` の前景・背景は `base.css` で semantic token に固定する。
- 切替 UI は設定ドロワーの `AppearancePanel`。
- 全セマンティックカラーを `tokens.css` の CSS 変数で切替える。light / dark どちらでも本文の可読性と focus リングの視認性を保つ。

### 8.2 ブレークポイント

shell の responsive は次の境界に統一済み（#325）。隣接レンジの `759/760`・`899/900`・`1099/1100` は重複しない min/max ペア。

| Name | レンジ | 境界 | 説明 |
|------|--------|------|------|
| mobile | `≤ 759px` | `max-width: 759` | 1 Column = 1 viewport。横 scroll snap と Control Center の direct jump を使う |
| small | `760–899px` | `min-width: 760` / `max-width: 899` | 狭い desktop。Column Canvas を開始し、actual Column width を縮小可能にする |
| medium | `900–1099px` | `min-width: 900` / `max-width: 1099` | 440px unit 2本を比較できる標準 desktop review range |
| large | `≥ 1100px` | `min-width: 1100` | 複数 Column と Stream / Metaverse wide surface の desktop range |

- 旧 `720px`（`base.css`）と `980px`（legacy）の外れ値、および `900` の off-by-one は上記境界へ寄せて統一した。
- ADR 0031 の Desktop / Mobile 境界は `759px / 760px` とする。現行 detail pane の `1100px` 境界は移行元 shell にだけ適用し、新しい Column kind や span の契約には使わない。

### 8.3 タッチターゲット

- 最小 44px × 44px（primary ボタンの `min-h-11`、icon ボタンの `size-10`+余白に一致）。

---

## 9. エージェント向けクイックリファレンス

```text
# kukuri は dark-first。値は apps/desktop/src/styles/tokens.css を正とする。
Primary / CTA (orange):  dark #f59d62 / light #d77d45   （hover dark #ee8f4e / light #c86f38）
Accent / focus (teal):   dark #00b3a4 / light #0f8c82
Background:              dark #101923 / light #f4efe6
Panel (base):           dark #0c1721 / light #ffffff     （accent / muted / soft の 4 段）
Foreground:             dark #f6f1e8 / light #21303b
Muted foreground:       dark #cbbdae / light #5f6c76
Border subtle:          dark #2a3a4a / light #cad3db
Focus ring:             dark rgba(0,179,164,0.45) / light rgba(15,140,130,0.32)
Destructive:            dark #ffb48a / light #b35f46

Radius:   input 14px / panel 22px / button pill(999px)
Shadow:   --shadow-panel（カード）/ --shadow-button-primary（CTA）
Font:     --font-sans = "IBM Plex Sans" → 和文(ヒラギノ/游ゴシック/Noto/メイリオ) → sans-serif
          --font-mono = "IBM Plex Mono","Cascadia Code","Consolas",monospace（code/kbd/samp + tabular-nums）
Theme:    <html data-theme="dark|light"> + localStorage "kukuri.desktop.theme"
```

### 短い指示例

```text
kukuri の DESIGN.md に従って UI を調整してください。
- 必ず apps/desktop/src/styles/tokens.css の CSS 変数から色・余白・radius・影を取る（hex 直書き禁止）。
- dark-first。primary action は warm-orange、accent/focus は cool-teal に限定する。
- 製品 UI と diagnostics UI を視覚階層で分離し、diagnostics は後景に置く。
- pubkey / ticket / hash は monospace + tabular-nums で表示する。
- loading / empty / error / success の各状態を定義する。
```
