# cn-admin-console

Community Node の運用管理向け Admin Console（React + TypeScript + Vite + shadcn/ui）。

## UI 方針

- `src/components/ui/` に shadcn/ui 構成の共通コンポーネントを配置
- 基本要素は `Button` / `Card` / `Badge` / `Input` / `Textarea` / `Select` / `Label` / `Notice` を利用

## 開発

```bash
pnpm dev
```

## ビルド

```bash
pnpm build
```

## 型チェック

```bash
pnpm typecheck
```

## UI テスト（Vitest + Testing Library）

```bash
pnpm test
```

テスト基盤:
- `vite.config.ts` の `test` 設定（`jsdom` / `setupFiles`）
- `src/test/setup.ts`（`@testing-library/jest-dom` と cleanup）
- `src/test/renderWithQueryClient.tsx`（React Query ラッパー）
