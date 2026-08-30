# kukuri UI/UX Design Contract

> kukuri desktop（Tauri + React + Tailwind v4 + shadcn/ui）で、利用者に何をどう見せ、どの品質を守るかを定義する製品固有の設計契約。

## この文書の位置づけ

- 本書はUIの規範となる設計契約である。作り方、証跡、変更種別ごとの検証は[`docs/adr/0014-uiux-dev-flow.md`](docs/adr/0014-uiux-dev-flow.md)に置く。
- CSSとstateの配置、import順、実装境界は[`docs/architecture/desktop-ui-implementation.md`](docs/architecture/desktop-ui-implementation.md)に置く。
- `apps/desktop/src/styles/tokens.css`は実際に実行される値、StorybookのFoundationsは描画確認面である。本書、実行値、確認面の食い違いは不具合として扱い、どれかを黙って正とせず同じ変更内で意図を明らかにして解消する。
- 本書の「現行契約」は実装に適用する。「提案」は追跡Issue、導入条件、移行範囲を持つ場合だけ記載でき、導入までは現行契約として扱わない。
- 変更判断の優先順位は、安全性・データ整合性・Accessibility、操作結果・状態継続・回復可能性、P2P／非同期状態、platform別のlayout／入力、性能、視覚的一貫性とkukuri固有性、motion／装飾の順とする。

## 1. 利用者、利用文脈、画面目的

kukuriの通常画面は、コンテンツを継続して閲覧・作成・操作する高密度なOperate型UIとする。マーケティング用landing pageの表現を通常画面へ持ち込まない。

| 利用者／文脈 | 主な目的 | UIが優先すること |
|---|---|---|
| 閲覧者 | topic、post、thread、profileを追う | 内容、投稿者、scope、取得状態を理解できること |
| 投稿者 | 投稿、返信、DM、channel内操作を完了する | 投稿先、draft、pending、成功、失敗、再試行を失わないこと |
| Stream／Metaverse参加者 | live sessionやDomeへ参加し操作する | resource状態、input ownership、退出／復帰、縮退状態を明示すること |
| 設定利用者 | account、community node、appearanceを管理する | 変更結果、同意、認証、再起動要否、回復方法を明示すること |
| 開発者／運用者 | diagnosticsを調査する | Developer modeまたは診断面に技術情報を集約し、通常導線を妨げないこと |

各surfaceは単一の主目的を持つ。

| Surface | 単一目的 | 主な操作 |
|---|---|---|
| Timeline／Bookmarks | 選択scopeの投稿または保存済み投稿を読む | 閲覧、投稿、返信、保存、scope切替 |
| Thread | 1つの会話文脈を追う | 返信、親投稿へ戻る、投稿者を開く |
| Profile | 1人の公開情報と投稿を理解する | 関係操作、投稿閲覧、自分の編集へ進む |
| Notifications | 自分に関係する変化を処理する | 対象文脈を開く、既読状態を理解する |
| Messages／Conversation | 相手を選び、privateな会話を継続する | 会話選択、送信、再試行 |
| Explore | topicや参加先を発見する | 検索、選択、参加、対象Columnを開く |
| Stream | live sessionを視聴・操作する | 再生、参加、fullscreen、退出 |
| Metaverse | Domeへ入り、空間を操作する | entry、focus取得、移動、fullscreen、退出 |
| Control Center／Settings | workspaceとapp設定を管理する | Column操作、認証、同意、設定変更、診断 |

## 2. 情報階層とkukuri固有性

- topic、post、thread、profile、channel、conversationを主役にする。node URL、peer id、ticket、hash、capability、sync内部状態は通常表示の主階層へ置かない。
- 技術識別子は通常画面では人が判別できる名前と短い補助情報に置き換える。完全値はcontext menu等の明示操作でコピー可能にし、Developer modeと診断画面では表示してよい。
- product UIとdiagnostics UIを視覚的・構造的に分け、diagnosticsはControl Center、Settings、inline Noticeの補助階層へ置く。
- warm-orangeをprimary action、cool-tealをaccent、focus、selected stateに限定する。dark-firstのdeep-navy、Column Canvas、topic-firstの情報構造をkukuri固有の基盤とする。
- 半透明gradient、過剰なcard nesting、装飾目的の巨大見出しで階層を作らず、solid surface、境界、余白、弱い拡散影で表す。
- 外部trendや一般的な禁止リストより、既存brief、token、component、受け入れ済みADRを優先する。

## 3. Column文脈と状態継続

- Columnのscope、target、active、focus、pin、preferred span、親子関係を別のstateとして扱う。activeとDOM focus、partially visibleを同一視しない。
- Column間を移動しても、入力中draft、選択中scope、会話文脈、未保存状態、session内scrollを不用意に失わない。
- 戻る操作は親Columnまたは直前の文脈へ戻し、無関係な既定画面へ飛ばさない。focusは移動元または操作を開始したcontrolへ復元する。
- canonical URLはfocus中Columnの共有targetだけを表す。Column配列、幅、順序、scroll位置、draftをURLへ載せない。
- local layoutとcanonical URLの責務は[`docs/adr/0031-variable-span-column-workspace.md`](docs/adr/0031-variable-span-column-workspace.md)に従う。

## 4. Componentと画面状態

### 4.1 Component状態

interactive componentは、該当する`default`、`hover`、`focus-visible`、`pressed`、`selected`、`disabled`、`pending`、`error`を定義する。

- 状態を色だけで区別しない。文字、icon、境界、形、accessible stateのいずれかを併用する。
- `disabled`と`pending`を混同しない。pendingは処理中であることと重複操作の扱いを伝える。
- errorは原因の要約と次に可能な行動を持つ。成功通知は実際に確定した操作名と一致させる。

### 4.2 画面状態

| 状態 | 表示契約 | 終端／回復 |
|---|---|---|
| initial loading | 初回取得中であることと対象を示す | success、empty、partial、offline、errorのいずれかへ必ず移る |
| refreshing | 直前の有効値を保持し、更新中を補助表示する | 新しい値または既存値を維持したerrorへ移る |
| empty | 取得成功かつ対象が0件の場合だけ表示する | 作成、参加、条件変更等の次の行動を示す |
| partial | 取得済み範囲と不足範囲を区別する | 追加取得、再試行、現状利用の選択肢を示す |
| offline | localで利用可能な値を保持し、network不在を示す | 再接続待ちまたは明示再試行を示す |
| reconnecting | 直前の値を保持し、接続回復中を示す | connected、degraded、offlineのいずれかへ移る |
| degraded | 利用できる機能と利用できない機能を分ける | 影響範囲と回復方法を示す |
| permission denied | 拒否された対象と理由の安全な要約を示す | 戻る、権限取得、設定変更のうち可能な行動を示す |
| error | 既存の有効値を消さず、失敗した部分を局所化する | retry、戻る、設定確認等の行動を示す |
| retry | 同じ操作を安全に再実行できることを示す | 重複副作用を起こさずsuccessまたはerrorへ移る |
| success | 完了した対象と結果を示す | 次の通常状態へ戻る |

false empty、無期限skeleton、取得不能な補助面が主要面を占有し続ける状態を禁止する。P2Pアプリでは「peerがいない」「まだ届いていない」「local cacheだけ」「一部取得済み」を0件と同一表示にしない。

## 5. 内容、国際化、文言

- user-generated contentはshort、normal、very long、emptyを確認する。日本語、英語、中国語、長いURL、絵文字、技術識別子、改行、添付あり／なしを含める。
- 長い語や識別子は`overflow-wrap: anywhere`等でcontainmentを守る。省略時は完全値へ到達できる手段を持つ。
- 日本語localeでは、kukuri、固有名、技術識別子以外の未意図な英語を混ぜない。他localeも同一情報と操作結果を保持する。
- ボタン、pending表示、成功通知、errorで同じ操作名を使う。曖昧な「実行」「失敗」だけで終わらせない。
- emptyとerrorには、利用者が次に行える具体的な行動を示す。値が空であることと取得できなかったことを区別する。
- icon-only操作はローカライズ済みの操作名をaccessible nameとtooltipの双方に使い、tooltipをpointer hoverとkeyboard focusの両方で表示する。

## 6. Accessibilityと入力

WCAG 2.2 AAを基準とする。自動検査の満点だけを適合の証明にせず、描画、keyboard、pointer、touch、screen readerの観測を組み合わせる。

- 通常文字は4.5:1以上、大きい文字と意味を持つ非text UIは3:1以上のcontrastを持つ。theme、hover、disabled、selected、focusを含む実際のforeground／background pairで確認する。
- semantic HTMLを優先し、roleとARIAはnative semanticsを補う場合だけ使う。見出し、landmark、label、description、errorの関係を保つ。
- focus順は視覚順と操作順に一致させ、DialogやColumn移動後にfocusを復元する。sticky header、footer、overlayでfocusを隠さない。
- keyboard trapを禁止する。Escape、Tab、矢印、Enter、Space等はcomponentの既知の操作モデルに合わせる。
- drag、swipe、pinchにはclick、tap、keyboardの代替を提供する。Column並べ替えは実pointer操作とkeyboard代替を持つ。
- 状態更新を必要に応じてlive regionで伝える。過剰なannounceや同じ通知の反復を避ける。
- 200% zoom／reflow、Windows High Contrast、screen reader、reduced motionは、影響する変更の手動確認対象とする。

### 6.1 操作領域profile

- WCAG 2.2 AAの最小targetは24×24 CSS pxとし、隣接targetとのspacingおよび例外条件を含めて判断する。
- touch向けの目標は44×44 CSS pxとする。Mobileのprimary action、page indicator、icon-only controlはこの目標を優先する。
- Desktopの高密度UIでは36px／40pxの見た目を許容できるが、実hit area、spacing、誤操作リスク、同等操作を確認し、24px未満にしない。

## 7. Responsiveとplatform profile

| Profile | Layout | 入力と密度 |
|---|---|---|
| Mobile（759px以下） | 1 Column＝1 viewport、horizontal scroll snap、safe areaを確保 | touch優先、44px目標、edge／indicatorがpaging gestureを所有 |
| Small Desktop（760〜899px） | Column Canvasを維持し、Column実幅を縮小可能 | pointer／keyboard併用、高密度だがdocument-level overflowを出さない |
| Medium Desktop（900〜1099px） | 440px Columnを比較できる標準確認幅 | 複数Column間のfocus、drag、keyboard移動を確認 |
| Large Desktop（1100px以上） | 複数Columnとwide surfaceを表示 | 情報を無制限に横へ広げず、Column単位の読解幅を保つ |

- desktop Column unitは`--column-unit`、gapは`--column-gap`を使い、複数spanの式は`width = span * columnUnit + (span - 1) * gap`とする。
- Timeline、Notifications、Profile、Threadは1 span、Messages／Conversationは1〜2、Streamは2、Metaverseは3、focused Metaverseは最大4を基準とする。
- internal layoutはColumn自身の実幅に応答し、viewportだけに依存しない。Column Canvasの意図的な横scrollは維持し、document-levelの横scrollを発生させない。
- overlay、Control Center、Composer、fullscreen controlはsafe areaと互いのhit areaを塞がない。
- Tauri／WebView依存surfaceはbrowserだけで完了とせず、影響するOS／WebViewでinput ownership、fullscreen、resource縮退を確認する。

## 8. Component設計

- 既存primitive、既存variant、composite component、新規primitiveの順で検討する。
- 同じpatternが2回以上現れる場合は共有を検討するが、見た目だけが似てdomain契約が異なるものを無理に統合しない。
- boolean propの増殖で暗黙の組合せを作らず、product上の意味を持つ明示variantまたはcompound componentを使う。
- shared componentはanatomy、variant、全状態、keyboard／pointer／touch、overflow、Storybook storyの契約を持つ。
- data取得、domain state、actionsとpresentationの境界を保ち、見た目の都合でdomain契約を変更しない。
- color、type、spacing、radius、border、shadow、motionは現行tokenから取る。1回しか使わないhelperや将来要件だけのadapterを増やさない。

## 9. Motion

- motionはstate変化、hierarchy、空間的な移動を説明する場合だけ使う。高頻度操作とkeyboard操作は即応性を優先する。
- `transition: all`を使わず、原則としてtransformとopacityを対象にする。操作で中断でき、最終stateが一意であること。
- `prefers-reduced-motion: reduce`とreview用`data-reduced-motion='reduce'`ではdurationを1ms、移動距離を0にする。focus、active、open／closed等の最終状態は省略しない。
- viewport端のdrag auto-scroll、scroll snap、fullscreen復帰で長いanimationを強制しない。

## 10. React／Tauri性能

- 独立した取得は直列化せず並行化する。partial failureを画面全体の失敗へ拡大しない。
- Stream／Metaverse等の重い機能は必要時に読み込み、画面外ではvideo停止、低品質化、render停止／低FPS等へ縮退する。network sessionとrender lifecycleを分離する。
- 長い一覧は実データで計測し、必要な場合だけvirtualizationまたは`content-visibility`を導入する。
- selectorなしの全store購読、不要な派生object、重複global event listener、render中のlayout read、無制限intervalを避ける。
- subscriptionとlistenerは所有者とcleanupを明確にし、再mountで重複しないことを確認する。
- localStorageはkeyとschema versionを持ち、不正値、旧version、書込み不能で主要操作を壊さない。
- Next.js、RSC、SSR、SEO固有の最適化は対象外とする。

## 11. 現行visual foundation

- dark-first。`<html data-theme='dark|light'>`で切り替え、OSの`prefers-color-scheme`へ自動追従しない。
- fontは`--font-sans`、技術識別子は`--font-mono`とtabular numeralsを使う。
- surfaceはbase、accent、muted、softの段階で構成し、primaryはwarm-orange、accent／focusはcool-teal、dangerはdestructive familyを使う。
- panelは`--radius-panel`、input／Noticeは`--radius-input`、pill controlは`--radius-pill`を使う。
- elevationは`--shadow-panel`、`--shadow-dropdown`、`--shadow-button-primary`に限定する。

### 11.1 現行token契約

次の表は`tokens.css`のroot／dark／lightで実行されるcustom propertyをミラーする。機械検査のため、scope、token、値の3列とmarkerを変更しない。`@theme inline`のTailwind aliasとreduced-motion overrideは派生値であり、この表の対象外とする。

<!-- TOKEN_CONTRACT_START -->
| Scope | Token | Value |
|---|---|---|
| global | `--font-sans` | `"IBM Plex Sans", "Hiragino Kaku Gothic ProN", "Yu Gothic", "Noto Sans JP", "Meiryo", "Segoe UI", sans-serif` |
| global | `--font-mono` | `"IBM Plex Mono", "Cascadia Code", "Consolas", SFMono-Regular, monospace` |
| global | `--text-display` | `clamp(1.9rem, 4vw, 3.5rem)` |
| global | `--text-h1` | `1.5rem` |
| global | `--text-h2` | `1.25rem` |
| global | `--text-h3` | `1rem` |
| global | `--text-body-reading` | `0.9375rem` |
| global | `--text-body` | `0.875rem` |
| global | `--text-caption` | `0.75rem` |
| global | `--radius-xs` | `0.5rem` |
| global | `--radius-sm` | `0.75rem` |
| global | `--radius` | `1rem` |
| global | `--radius-panel` | `22px` |
| global | `--radius-input` | `14px` |
| global | `--radius-pill` | `999px` |
| global | `--space-2xs` | `0.25rem` |
| global | `--space-xs` | `0.5rem` |
| global | `--space-sm` | `0.75rem` |
| global | `--space-md` | `1rem` |
| global | `--space-lg` | `1.5rem` |
| global | `--space-xl` | `2rem` |
| global | `--space-2xl` | `3rem` |
| global | `--column-unit` | `27.5rem` |
| global | `--column-gap` | `var(--space-md)` |
| global | `--motion-duration-fast` | `120ms` |
| global | `--motion-duration-standard` | `200ms` |
| global | `--motion-duration-slow` | `280ms` |
| global | `--motion-easing-standard` | `cubic-bezier(0.2, 0, 0, 1)` |
| global | `--motion-easing-enter` | `cubic-bezier(0, 0, 0, 1)` |
| global | `--motion-easing-exit` | `cubic-bezier(0.3, 0, 1, 1)` |
| global | `--motion-distance-column` | `1.5rem` |
| global | `--motion-distance-control-center` | `2rem` |
| global | `--blur-hud` | `14px` |
| global | `--surface-metaverse` | `#101318` |
| dark | `--background` | `#101923` |
| dark | `--shell-background` | `#101923` |
| dark | `--foreground` | `#f6f1e8` |
| dark | `--foreground-strong` | `#fff7ef` |
| dark | `--muted-foreground` | `#cbbdae` |
| dark | `--muted-foreground-soft` | `#a89b8f` |
| dark | `--surface-panel` | `#0c1721` |
| dark | `--surface-panel-solid` | `#0c1721` |
| dark | `--surface-panel-accent` | `#162231` |
| dark | `--surface-panel-muted` | `#13202c` |
| dark | `--surface-panel-soft` | `#182632` |
| dark | `--surface-input` | `#101b26` |
| dark | `--surface-raised` | `#1b2a36` |
| dark | `--surface-button-primary` | `#f59d62` |
| dark | `--surface-button-primary-hover` | `#ee8f4e` |
| dark | `--surface-button-secondary` | `#233241` |
| dark | `--surface-button-ghost` | `#1a2734` |
| dark | `--surface-button-ghost-hover` | `#223241` |
| dark | `--surface-active` | `#17393c` |
| dark | `--surface-overlay` | `#071019` |
| dark | `--surface-avatar` | `#21303d` |
| dark | `--surface-media-loading` | `#1a2734` |
| dark | `--surface-media-ready` | `#173439` |
| dark | `--surface-skeleton` | `#243442` |
| dark | `--surface-selection` | `#d98b55` |
| dark | `--surface-accent-soft` | `#17393c` |
| dark | `--surface-warning-soft` | `#463423` |
| dark | `--surface-destructive-soft` | `#4a2b22` |
| dark | `--surface-info-soft` | `#203449` |
| dark | `--surface-badge-neutral` | `#1a2734` |
| dark | `--surface-contrast` | `#20303c` |
| dark | `--border-subtle` | `#2a3a4a` |
| dark | `--border-subtle-strong` | `#39495a` |
| dark | `--border-accent` | `#2d7b76` |
| dark | `--border-warning` | `#a36b40` |
| dark | `--border-destructive` | `#a35e49` |
| dark | `--primary-start` | `#f59d62` |
| dark | `--primary-end` | `#f59d62` |
| dark | `--primary-foreground` | `#0e1b26` |
| dark | `--accent` | `#00b3a4` |
| dark | `--accent-foreground` | `#eafffb` |
| dark | `--destructive` | `#ffb48a` |
| dark | `--warning` | `#e6b066` |
| dark | `--danger` | `#ffb48a` |
| dark | `--ring` | `rgba(0, 179, 164, 0.45)` |
| dark | `--shadow-panel` | `0 18px 60px rgba(2, 7, 15, 0.22)` |
| dark | `--shadow-dropdown` | `0 12px 32px rgba(2, 7, 15, 0.12)` |
| dark | `--shadow-button-primary` | `0 10px 28px rgba(245, 157, 98, 0.16)` |
| dark | `--scrollbar-track` | `#12202c` |
| dark | `--scrollbar-thumb` | `#2a4d56` |
| dark | `--scrollbar-thumb-hover` | `#38717c` |
| light | `--background` | `#f4efe6` |
| light | `--shell-background` | `#f4efe6` |
| light | `--foreground` | `#21303b` |
| light | `--foreground-strong` | `#15202a` |
| light | `--muted-foreground` | `#5f6c76` |
| light | `--muted-foreground-soft` | `#74818a` |
| light | `--surface-panel` | `#ffffff` |
| light | `--surface-panel-solid` | `#ffffff` |
| light | `--surface-panel-accent` | `#f5ede2` |
| light | `--surface-panel-muted` | `#edf2f6` |
| light | `--surface-panel-soft` | `#e6edf2` |
| light | `--surface-input` | `#f8f4ee` |
| light | `--surface-raised` | `#dde5ec` |
| light | `--surface-button-primary` | `#d77d45` |
| light | `--surface-button-primary-hover` | `#c86f38` |
| light | `--surface-button-secondary` | `#dfe6ec` |
| light | `--surface-button-ghost` | `#edf2f6` |
| light | `--surface-button-ghost-hover` | `#e3ebf1` |
| light | `--surface-active` | `#d8eee9` |
| light | `--surface-overlay` | `#d7dfe7` |
| light | `--surface-avatar` | `#dfe8ee` |
| light | `--surface-media-loading` | `#dde5ec` |
| light | `--surface-media-ready` | `#d8eee9` |
| light | `--surface-skeleton` | `#e8eef3` |
| light | `--surface-selection` | `#e9b28c` |
| light | `--surface-accent-soft` | `#d8eee9` |
| light | `--surface-warning-soft` | `#f6e7d9` |
| light | `--surface-destructive-soft` | `#f6dfd4` |
| light | `--surface-info-soft` | `#dce7f4` |
| light | `--surface-badge-neutral` | `#edf2f6` |
| light | `--surface-contrast` | `#dde5ec` |
| light | `--border-subtle` | `#cad3db` |
| light | `--border-subtle-strong` | `#b7c2cb` |
| light | `--border-accent` | `#78a8a2` |
| light | `--border-warning` | `#d1a06d` |
| light | `--border-destructive` | `#d89b86` |
| light | `--primary-start` | `#d77d45` |
| light | `--primary-end` | `#d77d45` |
| light | `--primary-foreground` | `#fff7ef` |
| light | `--accent` | `#0f8c82` |
| light | `--accent-foreground` | `#143633` |
| light | `--destructive` | `#b35f46` |
| light | `--warning` | `#9a6e2a` |
| light | `--danger` | `#b35f46` |
| light | `--ring` | `rgba(15, 140, 130, 0.32)` |
| light | `--shadow-panel` | `0 18px 48px rgba(33, 48, 59, 0.12)` |
| light | `--shadow-dropdown` | `0 12px 32px rgba(33, 48, 59, 0.1)` |
| light | `--shadow-button-primary` | `0 10px 24px rgba(215, 125, 69, 0.18)` |
| light | `--scrollbar-track` | `#edf2f6` |
| light | `--scrollbar-thumb` | `#b8c6d2` |
| light | `--scrollbar-thumb-hover` | `#93a8b8` |
<!-- TOKEN_CONTRACT_END -->

### 11.2 提案token

現時点で提案tokenはない。追加する場合は現行契約表へ混ぜず、追跡Issue、導入条件、consumer、dark／light値をこの節に記載し、導入時に現行契約へ移す。

## 12. エージェント向けクイックリファレンス

```text
kukuriはdark-firstのOperate型UI。
topic／contentを主役にし、diagnosticsと完全な技術識別子は後景へ置く。
色、余白、radius、shadow、motionはtokens.cssの現行tokenを使う。
partial／offline／reconnecting／degradedをemptyと区別し、直前の有効値を保持する。
Columnのscope、focus、draft、戻る文脈を不用意に失わない。
pointer、touch、keyboard、screen readerで同じ操作結果へ到達できるようにする。
長文、空値、各locale、狭幅、200% zoom、reduced motionを変更範囲に応じて確認する。
実装と検証の手順はADR 0014、CSS／stateの配置はdesktop UI architectureを参照する。
```
