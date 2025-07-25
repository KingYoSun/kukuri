import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Header } from '../Header'

describe('Header', () => {
  it('ヘッダーの基本要素が表示されること', () => {
    render(<Header />)

    // ロゴが表示されること
    expect(screen.getByText('kukuri')).toBeInTheDocument()
    
    // 通知ボタンが存在すること
    const notificationButton = screen.getByRole('button', { name: /通知/i })
    expect(notificationButton).toBeInTheDocument()
    
    // アバターが表示されること
    expect(screen.getByText('U')).toBeInTheDocument()
  })

  it('ユーザーメニューが正しく動作すること', async () => {
    const user = userEvent.setup()
    render(<Header />)

    // 初期状態ではメニューが非表示
    expect(screen.queryByText('マイアカウント')).not.toBeInTheDocument()
    
    // アバターをクリックしてメニューを開く
    const avatarButton = screen.getByRole('button', { name: /U/i })
    await user.click(avatarButton)
    
    // メニューアイテムが表示されること
    expect(screen.getByText('マイアカウント')).toBeInTheDocument()
    expect(screen.getByText('設定')).toBeInTheDocument()
    expect(screen.getByText('ログアウト')).toBeInTheDocument()
  })

  it('通知ボタンがクリック可能であること', async () => {
    const user = userEvent.setup()
    render(<Header />)

    const notificationButton = screen.getByRole('button', { name: /通知/i })
    
    // クリックイベントのテスト（将来的に機能を追加する際の準備）
    await user.click(notificationButton)
    
    // ボタンが存在し、クリック可能であることを確認
    expect(notificationButton).toBeEnabled()
  })

  it('適切なスタイリングが適用されていること', () => {
    const { container } = render(<Header />)
    
    const header = container.querySelector('header')
    expect(header).toHaveClass('h-16', 'border-b', 'bg-background')
  })
})