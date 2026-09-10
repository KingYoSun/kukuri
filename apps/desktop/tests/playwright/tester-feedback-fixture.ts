import type { Page } from '@playwright/test';

export async function seedFeedback(page: Page, locale: 'en' | 'ja' | 'zh-CN', theme: string) {
  await page.addInitScript(({ locale, theme }) => {
    localStorage.setItem('kukuri.desktop.locale', locale);
    localStorage.setItem('kukuri.desktop.theme', theme);
    localStorage.setItem('kukuri.desktop.developer-mode', 'false');
    const fixture = { supported: false, calls: [] as string[] };
    Object.defineProperty(window, '__feedbackFixture', { value: fixture });
    let desktopApi = window.__KUKURI_DESKTOP__;
    Object.defineProperty(window, '__KUKURI_DESKTOP__', {
      configurable: true,
      get: () => desktopApi,
      set: (api: typeof desktopApi) => {
        desktopApi = api;
        if (!api) return;
        const node = 'https://feedback.example';
        const authenticate = api.authenticateCommunityNode.bind(api);
        const accept = api.acceptCommunityNodeConsents.bind(api);
        const policies = api.fetchCommunityNodePolicies.bind(api);
        const setup = (async () => {
          await api.setCommunityNodeConfig([{ base_url: node }]);
          await authenticate(node);
          await accept(node, (await policies(node)).policies, locale);
        })();
        const config = api.getCommunityNodeConfig.bind(api);
        api.getCommunityNodeConfig = async () => { await setup; return config(); };
        const statuses = api.getCommunityNodeStatuses.bind(api);
        api.getCommunityNodeStatuses = async () => { await setup; return statuses(); };
        const manifest = api.fetchCommunityNodeManifest.bind(api);
        api.fetchCommunityNodeManifest = async (baseUrl) => {
          await setup;
          const result = await manifest(baseUrl);
          return { ...result, manifest: { ...result.manifest!, node_name: 'Search / 検索 / 搜索',
            capability_scope: { available_enabled: fixture.supported ? ['community_index', 'tester_feedback'] : ['community_index'], planned_enabled: [] },
          } };
        };
        api.submitCommunityNodeTesterFeedback = async () => { fixture.calls.push('submit'); return { reference_id: 'fixture' }; };
        api.authenticateCommunityNode = async (...args) => { fixture.calls.push('authenticate'); return authenticate(...args); };
        api.acceptCommunityNodeConsents = async (...args) => { fixture.calls.push('accept'); return accept(...args); };
        const save = api.setCommunityNodeConfig.bind(api);
        api.setCommunityNodeConfig = async (...args) => { fixture.calls.push('save'); return save(...args); };
      },
    });
  }, { locale, theme });
}
