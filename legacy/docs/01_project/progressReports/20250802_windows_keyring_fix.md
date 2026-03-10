# Windows環境でのアカウント永続化問題の修正（第2版）

## 概要
Windows環境で新規アカウント作成後にリロードしてもログイン状態が維持されない問題を修正しました。

## 問題の詳細
- **現象**: アカウント作成は成功するが、リロード後に「No metadata entry found in keyring」となる
- **原因**: Windows環境でのEntry::new_with_target()の使い方が複雑すぎた

## 修正内容

### 1. シンプルなアプローチへの変更
- `Entry::new_with_target()`の使用を廃止
- 全プラットフォームで統一的に`Entry::new()`を使用
- Windows専用コードを削除してシンプル化

### 2. windows-nativeフィーチャーの有効化
```toml
# Cargo.toml
keyring = { version = "3.6.3", features = ["windows-native"] }
```

### 3. 主な変更点
```rust
// 以前の複雑な実装を削除
// 全プラットフォームで統一的なシンプルな実装に変更
let entry = Entry::new(SERVICE_NAME, ACCOUNTS_KEY).context("Failed to create keyring entry")?;
```

### 4. デバッグ機能の簡略化
- 保存直後の読み取りテストを全プラットフォームで実行
- Windows固有の条件分岐を削除

## 影響範囲
- `src/modules/secure_storage/mod.rs`: シンプルな実装に変更
- `Cargo.toml`: windows-nativeフィーチャーを追加

## テスト方法
1. `pnpm tauri dev`で開発環境を起動
2. 新規アカウントを作成
3. ページをリロード
4. ログイン状態が維持されていることを確認

## 参考資料
- [keyring crate documentation](https://docs.rs/keyring/latest/keyring/)

## 作成日
2025年08月02日