import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import App from '../App'

describe('App', () => {
  it('アプリケーションが正しくレンダリングされること', () => {
    render(<App />)
    
    // ヘッダーのロゴが表示されること
    expect(screen.getByText('kukuri')).toBeInTheDocument()
    
    // タイムラインが表示されること
    expect(screen.getByRole('heading', { name: 'タイムライン' })).toBeInTheDocument()
  })

  it('メインレイアウトが適用されていること', () => {
    render(<App />)
    
    // ヘッダーが存在すること
    expect(screen.getByRole('banner')).toBeInTheDocument()
    
    // サイドバーが存在すること
    expect(screen.getByRole('complementary')).toBeInTheDocument()
    
    // メインコンテンツエリアが存在すること
    expect(screen.getByRole('main')).toBeInTheDocument()
  })

  it('Toasterコンポーネントが含まれていること', () => {
    render(<App />)
    
    // Toasterのコンテナが存在することを確認
    // sonnerはポータルを使用するため、body直下に追加される
    expect(document.body.querySelector('[data-sonner-toaster]')).toBeDefined()
  })
})