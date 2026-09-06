import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';

import { invokeDesktop } from '@/lib/api/invoke/desktop';
import { RELEASE_LATEST_URL } from '@/lib/releaseReadiness';
import { createDesktopShellStore, DesktopShellStoreContext } from '@/shell/store';
import { ReleasePanel } from './ReleasePanel';
import i18n from '@/i18n';

vi.mock('@/lib/api/invoke/desktop', () => ({ invokeDesktop: vi.fn().mockResolvedValue('available') }));
vi.mock('@/lib/releaseReadiness', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/releaseReadiness')>()),
  isTauriRuntime: () => true,
}));
afterEach(() => vi.clearAllMocks());

test('native release link asks the OS browser to open the existing destination', async () => {
  render(
    <DesktopShellStoreContext.Provider value={createDesktopShellStore()}>
      <ReleasePanel showDiagnostics={false} />
    </DesktopShellStoreContext.Provider>
  );
  fireEvent.click(screen.getByRole('link', { name: /latest release/i }));
  await waitFor(() => expect(invokeDesktop).toHaveBeenCalledWith('open_external_url', {
    url: RELEASE_LATEST_URL,
  }));
});

test('all release resource and feedback anchors use the same native route', async () => {
  render(
    <DesktopShellStoreContext.Provider value={createDesktopShellStore()}>
      <ReleasePanel />
    </DesktopShellStoreContext.Provider>
  );
  const links = screen.getAllByRole('link');
  expect(links).toHaveLength(6); // destination, four resources, feedback
  for (const link of links) {
    vi.mocked(invokeDesktop).mockClear();
    fireEvent.click(link);
    await waitFor(() => expect(link).not.toHaveAttribute('aria-disabled'));
    expect(invokeDesktop).toHaveBeenCalledWith('open_external_url', { url: link.getAttribute('href') });
  }
});

test.each(['en', 'ja', 'zh-CN'])('browser failure is localized without raw native error in %s', async (locale) => {
  await i18n.changeLanguage(locale);
  vi.mocked(invokeDesktop).mockImplementation(async (command) => {
    if (command === 'open_external_url') throw new Error('raw secret fixture');
    return 'available';
  });
  render(
    <DesktopShellStoreContext.Provider value={createDesktopShellStore()}>
      <ReleasePanel showDiagnostics={false} />
    </DesktopShellStoreContext.Provider>
  );
  fireEvent.click(screen.getByRole('link', { name: i18n.t('settings:release.resources.latestRelease') }));
  expect(await screen.findByRole('alert')).toHaveTextContent(i18n.t('common:externalLink.failed'));
  expect(screen.queryByText(/raw secret/)).not.toBeInTheDocument();
});
