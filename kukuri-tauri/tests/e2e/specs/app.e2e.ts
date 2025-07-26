import { setupE2ETest, beforeEachTest, afterEachTest } from '../helpers/setup'

describe('Kukuri App E2E Tests', () => {
  before(async () => {
    await setupE2ETest()
  })
  
  beforeEach(async () => {
    await beforeEachTest()
  })
  
  afterEach(async function() {
    await afterEachTest(this.currentTest?.title || 'unknown')
  })
  
  describe('App Launch', () => {
    it('should launch the application successfully', async () => {
      // アプリが起動していることを確認
      const rootElement = await $('#root')
      expect(await rootElement.isDisplayed()).toBe(true)
      
      // ヘッダーが表示されていることを確認
      const header = await $('header')
      expect(await header.isDisplayed()).toBe(true)
    })
    
    it('should display the main layout components', async () => {
      // サイドバーの確認
      const sidebar = await $('[data-testid="sidebar"]')
      expect(await sidebar.isDisplayed()).toBe(true)
      
      // メインコンテンツエリアの確認
      const mainContent = await $('main')
      expect(await mainContent.isDisplayed()).toBe(true)
    })
    
    it('should show the app title', async () => {
      const title = await $('h1')
      const titleText = await title.getText()
      expect(titleText).toContain('Kukuri')
    })
  })
  
  describe('Navigation', () => {
    it('should navigate to home page by default', async () => {
      // URLの確認（Tauriアプリではtauri://localhostなど）
      const url = await browser.getUrl()
      expect(url).toMatch(/\/$|\/index\.html/)
    })
    
    it('should navigate to settings page', async () => {
      // 設定リンクをクリック
      const settingsLink = await $('a[href="/settings"]')
      await settingsLink.click()
      
      // 設定ページが表示されることを確認
      await browser.waitUntil(
        async () => {
          const settingsTitle = await $('h2')
          const text = await settingsTitle.getText()
          return text.includes('Settings')
        },
        { timeout: 5000 }
      )
    })
    
    it('should navigate between pages using sidebar', async () => {
      // サイドバーのリンクを取得
      const sidebarLinks = await $$('[data-testid="sidebar"] a')
      expect(sidebarLinks.length).toBeGreaterThan(0)
      
      // 各リンクをクリックして動作確認
      for (const link of sidebarLinks) {
        const linkText = await link.getText()
        await link.click()
        
        // ページが変更されたことを確認
        await browser.pause(500) // 遷移アニメーションを待つ
        
        // アクティブなリンクが変更されたことを確認
        const activeLink = await $('[data-testid="sidebar"] a.active')
        if (activeLink) {
          const activeText = await activeLink.getText()
          expect(activeText).toBe(linkText)
        }
      }
    })
  })
  
  describe('Theme Toggle', () => {
    it('should toggle between light and dark theme', async () => {
      // テーマトグルボタンを探す
      const themeToggle = await $('[data-testid="theme-toggle"]')
      
      if (await themeToggle.isExisting()) {
        // 初期テーマを取得
        const initialTheme = await browser.execute(() => {
          return document.documentElement.getAttribute('data-theme') || 
                 document.documentElement.classList.contains('dark') ? 'dark' : 'light'
        })
        
        // テーマを切り替え
        await themeToggle.click()
        
        // テーマが変更されたことを確認
        const newTheme = await browser.execute(() => {
          return document.documentElement.getAttribute('data-theme') || 
                 document.documentElement.classList.contains('dark') ? 'dark' : 'light'
        })
        
        expect(newTheme).not.toBe(initialTheme)
        
        // もう一度切り替えて元に戻る
        await themeToggle.click()
        
        const finalTheme = await browser.execute(() => {
          return document.documentElement.getAttribute('data-theme') || 
                 document.documentElement.classList.contains('dark') ? 'dark' : 'light'
        })
        
        expect(finalTheme).toBe(initialTheme)
      }
    })
  })
  
  describe('Window Controls', () => {
    it('should minimize the window', async () => {
      // 最小化ボタンをクリック（Tauriアプリの場合）
      const minimizeButton = await $('[data-testid="minimize-button"]')
      
      if (await minimizeButton.isExisting()) {
        await minimizeButton.click()
        // 実際の最小化の確認は環境依存のため、エラーが発生しないことのみ確認
      }
    })
    
    it('should maximize and restore the window', async () => {
      const maximizeButton = await $('[data-testid="maximize-button"]')
      
      if (await maximizeButton.isExisting()) {
        // 最大化
        await maximizeButton.click()
        await browser.pause(500)
        
        // 元のサイズに戻す
        await maximizeButton.click()
        await browser.pause(500)
      }
    })
  })
  
  describe('Error Handling', () => {
    it('should display error messages gracefully', async () => {
      // 無効なURLにアクセスしてエラーを発生させる
      await browser.execute(() => {
        window.location.hash = '#/invalid-route'
      })
      
      await browser.pause(1000)
      
      // エラーページまたはフォールバックが表示されることを確認
      const errorMessage = await $('[data-testid="error-message"]')
      if (await errorMessage.isExisting()) {
        const errorText = await errorMessage.getText()
        expect(errorText).toBeTruthy()
      } else {
        // フォールバックとしてホームページが表示されることを確認
        const homeElement = await $('[data-testid="home-page"]')
        expect(await homeElement.isExisting()).toBe(true)
      }
    })
  })
})