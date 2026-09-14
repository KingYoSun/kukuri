// Shared by browser/visual tests and the local native-WebView verification server.
// This function is self-contained so Playwright can serialize it into the page.
export function installReplyLayoutFixture() {
  let api: typeof window.__KUKURI_DESKTOP__;
  Object.defineProperty(window, '__KUKURI_DESKTOP__', {
    configurable: true,
    get: () => api,
    set: (value: NonNullable<typeof window.__KUKURI_DESKTOP__>) => {
      const original = value.listTimeline.bind(value);
      value.listTimeline = async (...args) => {
        const result = await original(...args);
        const base = result.items[0];
        if (!base) return result;
        return { ...result, items: [{
          ...base,
          object_id: 'reply-layout-child',
          author_pubkey: 'c'.repeat(64), author_name: 'CliPeerA', author_display_name: 'CliPeerA',
          content: '@GrokTester reply from CliPeerA for notify test',
          created_at: 1789324321,
          root_id: 'reply-layout-root', reply_to: 'reply-layout-parent',
          reply_preview: {
            object_id: 'reply-layout-parent', topic: args[0],
            author: { pubkey: 'b'.repeat(64), name: 'GrokTester', display_name: 'GrokTester', picture_asset: null },
            content: '動画添付 UX テスト — 直前の返信対象。長い本文でも2行までの簡略表示で会話の関係を確認できます。'.repeat(3),
            attachments: [], root_id: 'reply-layout-root', reply_to: 'reply-layout-root',
          },
        }] };
      };
      api = value;
    },
  });
}
