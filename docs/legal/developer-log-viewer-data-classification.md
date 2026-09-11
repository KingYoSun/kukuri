# Feature Data Classification: 開発者向けアプリ内ログの閲覧・書き出し

ADR 0002 (`docs/adr/0002-feature-data-classification-template.md`) に基づく分類。Issue #978。

### Feature Data Classification
- Feature 名: 開発者モード向けアプリ内ログビューア(in-memory tracing ring buffer の閲覧・コピー・書き出し)
- Durable / Transient: Transient(process のメモリだけに保持し、終了で消える。file・DB・設定へ保存しない)
- Canonical Source: Tauri backend process の `tracing` event。標準出力用 subscriber と同じ `EnvFilter` を通った event だけを `apps/desktop/src-tauri/src/tracing.rs` の `DesktopLogBuffer` へ写す
- Replicated?: No(アプリはネットワークへ送らない。持ち出しは利用者のコピー／書き出し操作だけ)
- Rebuildable From: 再構築不可(過去の event は再取得できない)。標準出力を保存していればそこから同じ行を得られる
- Public Replica / Private Replica / Local Only: Local Only
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要
- SQLite projection 必要有無: 不要
- 必須 contract:
  - buffer の上限は 2,000 件 / 合計 1 MiB / 1 行 4 KiB で固定し、超過は古い行から落とす。新しい行は捨てず、標準出力へは影響しない(`tracing.rs` の unit test)。
  - `read_desktop_logs` / `set_developer_mode_enabled` は invoke gate を通り、Ready 以外では拒否される。`read_desktop_logs` は frontend からミラーされた開発者モードが OFF のあいだ `developer_mode_disabled` で拒否し、buffer に触れない(`commands/developer_logs.rs` の unit / IPC round-trip test、`invoke_gate.rs`)。
  - 秘密鍵、auth token、DM 本文、パスフレーズをログへ出力しない既存契約(`docs/legal/account-key-export-data-classification.md` ほか)を維持し、ビューアは event をそのまま表示するだけで新たな情報源を持たない。
  - `crates/kukuri-cli/command-parity.json` に `gui_diagnostics` として登録し、CLI の業務 command へは追加しない。
- 必須 scenario: 開発者モード OFF → 設定 > 開発者 に viewer が出ず IPC を呼ばない。ON → 表示時に 1 回取得、「ログを更新」で再取得、コピー／`kukuri-logs.txt` 書き出しは click 時だけ。OFF に戻すと viewer が消える(`SettingsPanels.test.tsx`、`DesktopShellPage.developerMode.test.tsx`、`tests/playwright/developer-mode.spec.ts`)。

## 補足
- ログ本文には peer / topic / channel / Node URL / local path などの識別子が含まれ得る。書き出しファイルとコピー本文の先頭に、含めない情報と共有前に不要な行を除く旨を明記する(`apps/desktop/src/lib/desktopLogs.ts` の `buildDesktopLogsExport`)。
- 開発者モードの正本は frontend の `localStorage`(`kukuri.desktop.developer-mode`)のまま。backend は in-memory ミラーだけを持ち、永続化しない。再起動・アカウント切替後の再 mount で frontend が送り直す。
- 起動失敗(Failed)状態ではログを読めない(INVAR-3)。必要なら別 Issue で扱う。
- 外部送信一覧は `docs/legal/app-data-flow-inventory.md` の「アプリ内ログ(開発者モード)」行を参照する。
