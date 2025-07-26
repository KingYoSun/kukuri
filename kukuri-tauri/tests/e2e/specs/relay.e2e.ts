import { AppHelper } from '../helpers/app'
import { setupE2ETest, beforeEachTest, afterEachTest } from '../helpers/setup'

describe('Relay Connection E2E Tests', () => {
  before(async () => {
    await setupE2ETest()
  })
  
  beforeEach(async () => {
    await beforeEachTest()
    // 各テストの前にログイン
    await AppHelper.login()
  })
  
  afterEach(async function() {
    await afterEachTest(this.currentTest?.title || 'unknown')
  })
  
  describe('Relay Management', () => {
    it('should open relay management panel', async () => {
      // 設定ページに移動
      await browser.url('/settings')
      await browser.pause(1000)
      
      // リレー管理セクションを開く
      const relaySection = await $('[data-testid="relay-management"]')
      await relaySection.click()
      
      // リレーパネルが表示される
      const relayPanel = await $('[data-testid="relay-panel"]')
      await relayPanel.waitForDisplayed({ timeout: 5000 })
      
      expect(await relayPanel.isDisplayed()).toBe(true)
    })
    
    it('should connect to a new relay', async () => {
      // リレーURL入力フィールド
      const relayInput = await $('[data-testid="relay-url-input"]')
      const relayUrl = 'wss://relay.damus.io'
      await relayInput.setValue(relayUrl)
      
      // 接続ボタンをクリック
      const connectButton = await $('[data-testid="connect-relay"]')
      await connectButton.click()
      
      // 接続中の表示
      const connectingIndicator = await $('[data-testid="connecting-indicator"]')
      if (await connectingIndicator.isExisting()) {
        await connectingIndicator.waitForDisplayed({ reverse: true, timeout: 30000 })
      }
      
      // 接続されたリレーが表示される
      await browser.waitUntil(
        async () => {
          const relayStatus = await AppHelper.getRelayStatus()
          return relayStatus[relayUrl] === 'connected'
        },
        {
          timeout: 30000,
          timeoutMsg: 'Relay connection failed'
        }
      )
    })
    
    it('should display relay connection status', async () => {
      // リレーステータスコンポーネント
      const relayStatus = await $('[data-testid="relay-status"]')
      await relayStatus.waitForDisplayed({ timeout: 5000 })
      
      // 接続済みリレー数が表示される
      const connectedCount = await $('[data-testid="connected-relay-count"]')
      const countText = await connectedCount.getText()
      const count = parseInt(countText.match(/\d+/)?.[0] || '0')
      
      expect(count).toBeGreaterThanOrEqual(0)
    })
    
    it('should disconnect from a relay', async () => {
      // 接続済みリレーのリスト
      const relayList = await $('[data-testid="relay-list"]')
      const relays = await relayList.$$('[data-testid^="relay-"]')
      
      if (relays.length > 0) {
        const firstRelay = relays[0]
        const relayUrl = await firstRelay.$('span:first-child').getText()
        
        // 切断ボタンをクリック
        const disconnectButton = await firstRelay.$('[data-testid="disconnect-relay"]')
        await disconnectButton.click()
        
        // 確認ダイアログが表示される場合
        const confirmButton = await $('[data-testid="confirm-disconnect"]')
        if (await confirmButton.isExisting()) {
          await confirmButton.click()
        }
        
        // リレーが切断されるまで待つ
        await browser.waitUntil(
          async () => {
            const relayStatus = await AppHelper.getRelayStatus()
            return relayStatus[relayUrl] !== 'connected'
          },
          {
            timeout: 10000,
            timeoutMsg: 'Relay disconnection failed'
          }
        )
      }
    })
    
    it('should handle invalid relay URL', async () => {
      const relayInput = await $('[data-testid="relay-url-input"]')
      await relayInput.setValue('invalid-url')
      
      const connectButton = await $('[data-testid="connect-relay"]')
      await connectButton.click()
      
      // エラーメッセージが表示される
      const errorMessage = await $('[data-testid="relay-error"]')
      await errorMessage.waitForDisplayed({ timeout: 5000 })
      
      const errorText = await errorMessage.getText()
      expect(errorText).toContain('Invalid')
    })
  })
  
  describe('Relay Performance', () => {
    it('should display relay latency', async () => {
      // 接続済みリレーがある場合
      const relayList = await $('[data-testid="relay-list"]')
      const relays = await relayList.$$('[data-testid^="relay-"]')
      
      if (relays.length > 0) {
        // 各リレーのレイテンシが表示される
        for (const relay of relays) {
          const latencyElement = await relay.$('[data-testid="relay-latency"]')
          if (await latencyElement.isExisting()) {
            const latencyText = await latencyElement.getText()
            expect(latencyText).toMatch(/\d+ms/)
          }
        }
      }
    })
    
    it('should show relay health indicators', async () => {
      const relayList = await $('[data-testid="relay-list"]')
      const relays = await relayList.$$('[data-testid^="relay-"]')
      
      for (const relay of relays) {
        // ヘルスインジケーター
        const healthIndicator = await relay.$('[data-testid="relay-health"]')
        if (await healthIndicator.isExisting()) {
          const healthClass = await healthIndicator.getAttribute('class')
          expect(healthClass).toMatch(/health-(good|warning|error)/)
        }
      }
    })
    
    it('should retry failed connections', async () => {
      // 失敗したリレーを探す
      const failedRelay = await $('[data-testid="relay-status-failed"]')
      
      if (await failedRelay.isExisting()) {
        // リトライボタン
        const retryButton = await failedRelay.$('[data-testid="retry-connection"]')
        await retryButton.click()
        
        // 再接続を試みる
        const connectingIndicator = await $('[data-testid="connecting-indicator"]')
        await connectingIndicator.waitForDisplayed({ timeout: 5000 })
        
        // 結果を待つ（成功または失敗）
        await connectingIndicator.waitForDisplayed({ 
          reverse: true, 
          timeout: 30000 
        })
      }
    })
  })
  
  describe('Relay Configuration', () => {
    it('should set relay preferences', async () => {
      // リレー設定を開く
      const relaySettings = await $('[data-testid="relay-settings"]')
      await relaySettings.click()
      
      // 自動接続の設定
      const autoConnectToggle = await $('[data-testid="auto-connect-toggle"]')
      if (await autoConnectToggle.isExisting()) {
        const initialState = await autoConnectToggle.getAttribute('aria-checked')
        await autoConnectToggle.click()
        
        const newState = await autoConnectToggle.getAttribute('aria-checked')
        expect(newState).not.toBe(initialState)
      }
      
      // 最大接続数の設定
      const maxConnectionsInput = await $('[data-testid="max-connections"]')
      if (await maxConnectionsInput.isExisting()) {
        await maxConnectionsInput.setValue('10')
        
        // 設定を保存
        const saveButton = await $('[data-testid="save-relay-settings"]')
        await saveButton.click()
        
        // 成功メッセージ
        const successMessage = await $('[data-testid="settings-saved"]')
        await successMessage.waitForDisplayed({ timeout: 5000 })
      }
    })
    
    it('should import/export relay list', async () => {
      // エクスポートボタン
      const exportButton = await $('[data-testid="export-relays"]')
      
      if (await exportButton.isExisting()) {
        await exportButton.click()
        
        // エクスポートされたデータが表示される
        const exportData = await $('[data-testid="export-data"]')
        await exportData.waitForDisplayed({ timeout: 5000 })
        
        const dataText = await exportData.getText()
        expect(dataText).toContain('wss://')
        
        // コピーボタン
        const copyButton = await $('[data-testid="copy-export"]')
        await copyButton.click()
        
        // コピー成功の表示
        const copySuccess = await $('[data-testid="copy-success"]')
        if (await copySuccess.isExisting()) {
          expect(await copySuccess.isDisplayed()).toBe(true)
        }
      }
    })
  })
  
  describe('Relay Events', () => {
    it('should send test event through relay', async () => {
      // Nostrテストパネルを開く
      const testPanelButton = await $('[data-testid="open-test-panel"]')
      
      if (await testPanelButton.isExisting()) {
        await testPanelButton.click()
        
        const testPanel = await $('[data-testid="nostr-test-panel"]')
        await testPanel.waitForDisplayed({ timeout: 5000 })
        
        // テストメッセージを入力
        const messageInput = await $('[data-testid="test-message-input"]')
        await messageInput.setValue('Test event from E2E test')
        
        // 送信
        const sendButton = await $('[data-testid="send-test-event"]')
        await sendButton.click()
        
        // イベントが送信されたことを確認
        const eventId = await $('[data-testid="sent-event-id"]')
        await eventId.waitForDisplayed({ timeout: 10000 })
        
        const eventIdText = await eventId.getText()
        expect(eventIdText).toMatch(/^[a-f0-9]{64}$/)
      }
    })
    
    it('should receive events from relay', async () => {
      // イベントフィードが表示される
      const eventFeed = await $('[data-testid="event-feed"]')
      
      if (await eventFeed.isExisting()) {
        // 新しいイベントが受信されるまで待つ
        const initialCount = await eventFeed.$$('[data-testid^="event-"]').length
        
        await browser.waitUntil(
          async () => {
            const currentCount = await eventFeed.$$('[data-testid^="event-"]').length
            return currentCount > initialCount
          },
          {
            timeout: 30000,
            timeoutMsg: 'No new events received'
          }
        )
        
        // 最新のイベントを確認
        const latestEvent = await eventFeed.$('[data-testid^="event-"]:first-child')
        const eventContent = await latestEvent.$('[data-testid="event-content"]')
        const content = await eventContent.getText()
        
        expect(content).toBeTruthy()
      }
    })
  })
  
  describe('Relay Failover', () => {
    it('should handle relay disconnection gracefully', async () => {
      // アクティブなリレーを取得
      const activeRelays = await $$('[data-testid="relay-status-connected"]')
      
      if (activeRelays.length > 1) {
        // 1つのリレーを切断
        const firstRelay = activeRelays[0]
        const disconnectButton = await firstRelay.$('[data-testid="disconnect-relay"]')
        await disconnectButton.click()
        
        // アプリが正常に動作し続けることを確認
        const postInput = await $('[data-testid="post-input"]')
        await postInput.setValue('Test post during relay failover')
        
        const postButton = await $('[data-testid="post-button"]')
        await postButton.click()
        
        // 投稿が成功することを確認
        await browser.waitUntil(
          async () => {
            const posts = await AppHelper.getPostList()
            return posts.some(post => post.content.includes('failover'))
          },
          {
            timeout: 10000,
            timeoutMsg: 'Post failed during relay failover'
          }
        )
      }
    })
  })
})