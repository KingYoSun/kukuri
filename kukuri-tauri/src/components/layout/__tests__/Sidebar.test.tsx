import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Sidebar } from '../Sidebar'

describe('Sidebar', () => {
  it('サイドバーの基本要素が表示されること', () => {
    render(<Sidebar />)

    // 新規投稿ボタンが表示されること
    expect(screen.getByRole('button', { name: /新規投稿/i })).toBeInTheDocument()
    
    // カテゴリーセクションが表示されること
    expect(screen.getByText('カテゴリー')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /トレンド/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /フォロー中/i })).toBeInTheDocument()
    
    // トピックセクションが表示されること
    expect(screen.getByText('参加中のトピック')).toBeInTheDocument()
  })

  it('トピックリストが正しく表示されること', () => {
    render(<Sidebar />)

    // 各トピックが表示されること
    expect(screen.getByRole('button', { name: /technology/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /programming/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /nostr/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /bitcoin/i })).toBeInTheDocument()
    
    // トピックの投稿数が表示されること
    expect(screen.getByText('1234')).toBeInTheDocument()
    expect(screen.getByText('892')).toBeInTheDocument()
    expect(screen.getByText('456')).toBeInTheDocument()
    expect(screen.getByText('789')).toBeInTheDocument()
  })

  it('新規投稿ボタンがクリック可能であること', async () => {
    const user = userEvent.setup()
    render(<Sidebar />)

    const newPostButton = screen.getByRole('button', { name: /新規投稿/i })
    
    // ボタンがクリック可能であることを確認
    await user.click(newPostButton)
    expect(newPostButton).toBeEnabled()
  })

  it('カテゴリーボタンがクリック可能であること', async () => {
    const user = userEvent.setup()
    render(<Sidebar />)

    const trendButton = screen.getByRole('button', { name: /トレンド/i })
    const followingButton = screen.getByRole('button', { name: /フォロー中/i })
    
    // 各ボタンがクリック可能であることを確認
    await user.click(trendButton)
    expect(trendButton).toBeEnabled()
    
    await user.click(followingButton)
    expect(followingButton).toBeEnabled()
  })

  it('トピックボタンがクリック可能であること', async () => {
    const user = userEvent.setup()
    render(<Sidebar />)

    const technologyButton = screen.getByRole('button', { name: /technology/i })
    
    // トピックボタンがクリック可能であることを確認
    await user.click(technologyButton)
    expect(technologyButton).toBeEnabled()
  })

  it('適切なスタイリングとレイアウトが適用されていること', () => {
    const { container } = render(<Sidebar />)
    
    const aside = container.querySelector('aside')
    expect(aside).toHaveClass('w-64', 'border-r', 'bg-background')
    
    // ScrollAreaが存在することを確認
    expect(container.querySelector('[data-radix-scroll-area-viewport]')).toBeInTheDocument()
  })
})