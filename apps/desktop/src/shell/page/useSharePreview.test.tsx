import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, test, vi } from 'vitest';

import { buildChannelAccessPreviewDeepLink } from '@/lib/internalLinks';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { useSharePreview } from '@/shell/page/useSharePreview';

const deepLinkMock = vi.hoisted(() => ({
  currentUrls: [] as string[],
  onOpenUrl: vi.fn(),
  unlisten: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-deep-link', () => ({
  getCurrent: vi.fn(async () => deepLinkMock.currentUrls),
  onOpenUrl: deepLinkMock.onOpenUrl,
}));

const preview = {
  channel_id: 'channel-imported',
  topic_id: 'kukuri:topic:private-imported',
  channel_label: 'Imported',
  inviter_pubkey: 'f'.repeat(64),
  owner_pubkey: 'f'.repeat(64),
  epoch_id: 'epoch-imported-1',
  expires_at: null,
  namespace_secret_hex: 'a'.repeat(64),
  kind: 'invite' as const,
};

function setup() {
  const api = createDesktopMockApi({ invitePreview: preview });
  const importChannelAccessToken = vi.fn().mockResolvedValue(undefined);
  const translate = vi.fn((key: string) => key);
  const hook = renderHook(() =>
    useSharePreview({ api, importChannelAccessToken, translate })
  );
  return { api, hook, importChannelAccessToken };
}

afterEach(() => {
  deepLinkMock.currentUrls = [];
  deepLinkMock.onOpenUrl.mockReset();
  deepLinkMock.unlisten.mockReset();
});

describe('useSharePreview', () => {
  test('opens a preview from a browser deep-link event and imports only after confirmation', async () => {
    deepLinkMock.onOpenUrl.mockResolvedValue(deepLinkMock.unlisten);
    const { api, hook, importChannelAccessToken } = setup();
    const previewSpy = vi.spyOn(api, 'previewChannelAccessToken');
    const token = 'invite:encoded-token';

    act(() => {
      window.dispatchEvent(
        new CustomEvent('kukuri:open-url', {
          detail: { url: buildChannelAccessPreviewDeepLink(token) },
        })
      );
    });

    await waitFor(() =>
      expect(hook.result.current.data).toMatchObject({
        channel_id: preview.channel_id,
        kind: preview.kind,
        topic_id: preview.topic_id,
      })
    );
    expect(previewSpy).toHaveBeenCalledWith(token);
    expect(importChannelAccessToken).not.toHaveBeenCalled();

    await act(async () => {
      await hook.result.current.confirmImport();
    });
    expect(importChannelAccessToken).toHaveBeenCalledWith(token);
    expect(hook.result.current.open).toBe(false);
    expect(hook.result.current.token).toBeNull();
  });

  test('consumes current Tauri URLs and releases both listeners on unmount', async () => {
    const token = 'invite:startup-token';
    deepLinkMock.currentUrls = [buildChannelAccessPreviewDeepLink(token)];
    deepLinkMock.onOpenUrl.mockResolvedValue(deepLinkMock.unlisten);
    const { api, hook } = setup();
    const previewSpy = vi.spyOn(api, 'previewChannelAccessToken');

    await waitFor(() => expect(previewSpy).toHaveBeenCalledWith(token));
    hook.unmount();
    expect(deepLinkMock.unlisten).toHaveBeenCalled();

    window.dispatchEvent(
      new CustomEvent('kukuri:open-url', {
        detail: { url: buildChannelAccessPreviewDeepLink('invite:after-unmount') },
      })
    );
    expect(previewSpy).toHaveBeenCalledTimes(1);
  });
});
