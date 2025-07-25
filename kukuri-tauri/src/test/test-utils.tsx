import { ReactElement } from 'react'
import { render, RenderOptions } from '@testing-library/react'

// カスタムレンダー関数（将来的にプロバイダーを追加する際に使用）
function customRender(
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'>,
) {
  return render(ui, {
    // 将来的にここにプロバイダーをラップする
    // wrapper: ({ children }) => <Providers>{children}</Providers>,
    ...options,
  })
}

// re-export everything
// eslint-disable-next-line react-refresh/only-export-components
export * from '@testing-library/react'
export { customRender as render }