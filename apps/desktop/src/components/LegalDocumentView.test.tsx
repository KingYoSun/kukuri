import { render, screen } from '@testing-library/react';
import { describe, expect, test } from 'vitest';

import { LegalDocumentView } from '@/components/LegalDocumentView';
import i18n, { type SupportedLocale } from '@/i18n';

const EXPECTED_TERMS_HEADINGS: Array<{
  locale: SupportedLocale;
  headings: string[];
}> = [
  {
    locale: 'ja',
    headings: [
      '投稿コンテンツの権利帰属',
      '必要な権利または許諾の保有表明',
      '限定的な利用許諾',
      '投稿撤回後の取扱い',
      '許諾に含まれない利用',
    ],
  },
  {
    locale: 'en',
    headings: [
      'Ownership of your content',
      'Your rights and permissions',
      'Limited technical license',
      'After withdrawal',
      'Uses not licensed',
    ],
  },
  {
    locale: 'zh-CN',
    headings: [
      '投稿内容的权利归属',
      '您对权利与许可的声明',
      '有限的技术性许可',
      '撤回投稿后的处理',
      '不在许可范围内的使用',
    ],
  },
];

describe.each(EXPECTED_TERMS_HEADINGS)('legal document in $locale', ({ locale, headings }) => {
  test('shows bundle version 2 and every content-rights clause', async () => {
    await i18n.changeLanguage(locale);
    render(<LegalDocumentView bundleVersion={2} />);

    expect(screen.getByText('v2')).toBeInTheDocument();
    for (const heading of headings) {
      expect(screen.getByRole('heading', { name: heading })).toBeInTheDocument();
    }
  });

  test('does not render a draft notice', async () => {
    await i18n.changeLanguage(locale);
    render(<LegalDocumentView bundleVersion={2} />);

    expect(screen.queryByText(/draft|ドラフト|草案/i)).not.toBeInTheDocument();
  });
});
