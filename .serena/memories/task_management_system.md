# タスク管理システム設計

## ディレクトリ構造

```
docs/01_project/activeContext/tasks/
├── README.md              # タスク管理の概要・ダッシュボード
├── priority/             # 優先度別タスク
│   ├── critical.md       # 今すぐ着手すべき最重要タスク（1-3個）
│   ├── high.md          # 高優先度タスク
│   └── medium.md        # 中優先度タスク
├── status/              # ステータス別タスク
│   ├── in_progress.md   # 作業中（ClaudeCodeが常に更新）
│   └── blocked.md       # ブロックされているタスク
├── completed/           # 完了タスク（日付別）
│   ├── 2025-01-15.md   # その日完了したタスク
│   ├── 2025-01-14.md   
│   └── archive/        # 1週間経過後に自動移動
│       └── 2025-01/    # 月別アーカイブ
├── metrics/            # メトリクス（自動更新用）
│   ├── build_status.md # ビルド状況
│   ├── test_results.md # テスト結果
│   └── code_quality.md # コード品質指標
└── context/           # コンテキスト情報
    ├── blockers.md    # 現在のブロッカー
    └── decisions.md   # 重要な決定事項

```

## 各ファイルの役割と更新ルール

### 1. README.md（ダッシュボード）
**自動生成・読み取り専用**
```markdown
# タスク管理ダッシュボード
最終更新: YYYY-MM-DD HH:MM

## 📍 現在のフォーカス
→ [critical.md](./priority/critical.md) を参照

## 📊 サマリー
- 最重要タスク: X個
- 進行中: X個  
- ブロッカー: X個
- 今日の完了: X個
```

### 2. priority/critical.md
**最大3個のタスク**
```markdown
# 最重要タスク
更新: YYYY-MM-DD

## 1. [タスク名]
- 期限: YYYY-MM-DD
- 担当: Claude/Human
- 次のアクション: [具体的な作業]
```

### 3. status/in_progress.md
**ClaudeCodeが作業するたびに更新**
```markdown
# 進行中タスク
更新: YYYY-MM-DD HH:MM

## [タスク名]
開始: YYYY-MM-DD HH:MM
進捗: 
- [x] ステップ1完了
- [ ] ステップ2実行中
最新状況: [1-2行]
```

### 4. completed/YYYY-MM-DD.md
**その日の完了タスクを追記**
```markdown
# YYYY年MM月DD日 完了タスク

## HH:MM - [タスク名]
- 成果: [簡潔な説明]
- 所要時間: X時間
- 関連PR/コミット: [リンク]
```

## 更新フロー

### ClaudeCodeの自然な動作
1. **タスク開始時**
   - `priority/critical.md`から1つ選択
   - `status/in_progress.md`に移動
   - TodoWriteツールと連動

2. **作業中**
   - `status/in_progress.md`のみ更新
   - 他のファイルに影響なし

3. **タスク完了時**
   - `completed/YYYY-MM-DD.md`に追記（新規作成も可）
   - `status/in_progress.md`から削除
   - `priority/`から次のタスクを選択

4. **ブロッカー発生時**
   - `context/blockers.md`に追記
   - `status/blocked.md`に移動

## 利点

### 1. 部分更新の影響を最小化
- 各ファイルが独立
- 更新失敗しても他に影響なし

### 2. 履歴の自然な保存
- 完了タスクは日付別ファイル
- 追記のみで更新不要

### 3. 並行作業の対応
- 複数のClaudeインスタンスが異なるファイルを更新可能

### 4. 検索性の向上
- ファイル名から内容が明確
- grepやfindが効率的

## 移行手順

### Phase 1: 構造作成
```bash
mkdir -p docs/01_project/activeContext/tasks/{priority,status,completed,metrics,context}
mkdir -p docs/01_project/activeContext/tasks/completed/archive
```

### Phase 2: 現在のタスクを分割
1. critical.mdに最重要3個を移動
2. in_progress.mdに進行中を移動  
3. 完了タスクを日付別に分割

### Phase 3: 自動化スクリプト
```python
# daily_cleanup.py
# - 1週間経過した完了ファイルをarchiveへ
# - README.mdのサマリー更新
# - metrics/の自動更新
```

## ClaudeCode用ルール（CLAUDE.mdに追記）

```markdown
## タスク管理ルール
- タスク開始時: `tasks/priority/critical.md`を確認
- 作業中: `tasks/status/in_progress.md`を更新
- 完了時: `tasks/completed/YYYY-MM-DD.md`に追記
- 新しい日付の場合は新規ファイル作成
- 他のファイルは必要時のみ更新
```