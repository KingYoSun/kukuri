import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Home } from '../Home'

describe('Home', () => {
  it('タイムラインのヘッダーが表示されること', () => {
    render(<Home />)
    
    expect(screen.getByRole('heading', { name: 'タイムライン' })).toBeInTheDocument()
  })

  it('投稿リストが正しく表示されること', () => {
    render(<Home />)
    
    // 投稿者の名前が表示されること
    expect(screen.getByText('Alice')).toBeInTheDocument()
    expect(screen.getByText('Bob')).toBeInTheDocument()
    
    // 投稿内容が表示されること
    expect(screen.getByText(/Nostrプロトコルを使った分散型SNS/)).toBeInTheDocument()
    expect(screen.getByText(/kukuriの開発進捗/)).toBeInTheDocument()
    
    // タイムスタンプが表示されること
    expect(screen.getByText('2時間前')).toBeInTheDocument()
    expect(screen.getByText('4時間前')).toBeInTheDocument()
  })

  it('投稿のインタラクションボタンが表示されること', () => {
    render(<Home />)
    
    // 各投稿に対してインタラクションボタンが存在すること（2投稿分）
    // 投稿1のボタン
    expect(screen.getByText('3')).toBeInTheDocument() // コメント数
    expect(screen.getByText('2')).toBeInTheDocument() // リポスト数
    expect(screen.getByText('12')).toBeInTheDocument() // いいね数
    
    // 投稿2のボタン
    expect(screen.getByText('8')).toBeInTheDocument() // コメント数
    expect(screen.getByText('5')).toBeInTheDocument() // リポスト数
    expect(screen.getByText('24')).toBeInTheDocument() // いいね数
    
    // シェアボタン（2つ）
    const shareButtons = screen.getAllByRole('button', { name: /share/i })
    expect(shareButtons).toHaveLength(2)
  })

  it('アバターが正しく表示されること', () => {
    render(<Home />)
    
    // アバターのフォールバックテキストが表示されること
    const avatarA = screen.getByText('A')
    const avatarB = screen.getByText('B')
    
    expect(avatarA).toBeInTheDocument()
    expect(avatarB).toBeInTheDocument()
  })

  it('公開鍵が表示されること', () => {
    render(<Home />)
    
    expect(screen.getByText('npub1...')).toBeInTheDocument()
    expect(screen.getByText('npub2...')).toBeInTheDocument()
  })

  it('インタラクションボタンがクリック可能であること', async () => {
    const user = userEvent.setup()
    render(<Home />)
    
    // 最初の投稿のいいねボタンをクリック
    const likeButtons = screen.getAllByRole('button', { name: /12|24/i })
    await user.click(likeButtons[0])
    
    // ボタンが有効であることを確認
    expect(likeButtons[0]).toBeEnabled()
  })

  it('投稿がカード形式で表示されること', () => {
    const { container } = render(<Home />)
    
    // カードコンポーネントが使用されていることを確認
    const cards = container.querySelectorAll('[data-testid="card"]')
    expect(cards.length).toBeGreaterThan(0)
  })
})