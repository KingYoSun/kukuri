import type { Page } from '@playwright/test';

export async function seedUnconsentedCommunityNodes(page: Page, {
  locale = 'en', theme = 'dark', failPoliciesOnce = false,
}: { locale?: string; theme?: string; failPoliciesOnce?: boolean } = {}) {
  await page.addInitScript(({ locale, theme, failPoliciesOnce }) => {
    localStorage.setItem('kukuri.desktop.locale', locale);
    localStorage.setItem('kukuri.desktop.theme', theme);
    const calls: string[] = [];
    Object.defineProperty(window, '__issue914Calls', { value: calls });
    let desktopApi = window.__KUKURI_DESKTOP__;
    Object.defineProperty(window, '__KUKURI_DESKTOP__', {
      configurable: true,
      get: () => desktopApi,
      set: (api: typeof desktopApi) => {
        desktopApi = api;
        if (!api) return;
        void api.setCommunityNodeConfig([{ base_url: 'https://first.example' }, { base_url: 'https://second.example' }]);
        let failed = false;
        const fetch = api.fetchCommunityNodePolicies.bind(api);
        api.fetchCommunityNodePolicies = async (baseUrl, language) => {
          calls.push(`policies:${baseUrl}`);
          if (failPoliciesOnce && !failed) { failed = true; throw new Error('offline fixture'); }
          // #1106: node が配信する Markdown 本文(見出し・箇条書き・引用・リンク)を再現する。
          const response = await fetch(baseUrl, language);
          return {
            ...response,
            policies: response.policies.map((policy) => policy.policy_slug === 'terms_of_service'
              ? {
                  ...policy,
                  body_markdown: [
                    policy.body_markdown,
                    '',
                    '> Note: generated from the operator config. This is not legal advice.',
                    '',
                    '## Scope',
                    '',
                    '- Applies only to features this node provides, such as `index` and `relay`.',
                    '- Contact: [operator support](https://example.com/support)',
                  ].join('\n'),
                }
              : policy),
          };
        };
        const accept = api.acceptCommunityNodeConsents.bind(api);
        api.acceptCommunityNodeConsents = async (...args) => {
          calls.push(`accept:${args[0]}`);
          return accept(...args);
        };
        const auth = api.authenticateCommunityNode.bind(api);
        api.authenticateCommunityNode = async (...args) => {
          calls.push(`auth:${args[0]}`);
          return auth(...args);
        };
      },
    });
  }, { locale, theme, failPoliciesOnce });
}

export async function communityNodeCalls(page: Page) {
  return page.evaluate(() => (window as unknown as { __issue914Calls: string[] }).__issue914Calls);
}
