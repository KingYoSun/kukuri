import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import type { DesktopApi } from '@/lib/api';

import { CommunityIndexingRequestDialog } from './CommunityIndexingRequestDialog';

test('public topic request omits private channel confirmation and capability', async () => {
  const submitCommunityNodeIndexingRequest = vi.fn().mockResolvedValue({
    request_id: 'request-1',
    status: 'pending',
  });
  render(
    <CommunityIndexingRequestDialog
      api={{ submitCommunityNodeIndexingRequest } as unknown as DesktopApi}
      target={{ kind: 'public_topic', topicId: 'kukuri:topic:demo' }}
      eligibleNodeBaseUrls={['https://index.example']}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );
  expect(screen.getByText(/A request does not guarantee indexing/)).toBeInTheDocument();

  fireEvent.click(screen.getByRole('button', { name: 'Submit request' }));

  await waitFor(() => expect(submitCommunityNodeIndexingRequest).toHaveBeenCalledTimes(1));
  expect(submitCommunityNodeIndexingRequest).toHaveBeenCalledWith({
    base_url: 'https://index.example',
    scope_kind: 'public_topic',
    topic_id: 'kukuri:topic:demo',
    channel_id: null,
    confirm_private_channel_secret_disclosure: false,
  });
  expect(await screen.findByText('The request is pending review.')).toBeInTheDocument();
});

test('private channel request stays disabled until explicit disclosure confirmation', async () => {
  const submitCommunityNodeIndexingRequest = vi.fn().mockResolvedValue({
    request_id: 'request-2',
    status: 'approved',
  });
  render(
    <CommunityIndexingRequestDialog
      api={{ submitCommunityNodeIndexingRequest } as unknown as DesktopApi}
      target={{
        kind: 'private_channel',
        topicId: 'kukuri:topic:demo',
        channelId: 'channel-1',
        channelLabel: 'Core',
      }}
      eligibleNodeBaseUrls={['https://index.example']}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );

  const submit = screen.getByRole('button', { name: 'Submit request' });
  expect(submit).toBeDisabled();
  fireEvent.click(
    screen.getByRole('checkbox', {
      name: /I agree to disclose this channel's read capability/,
    })
  );
  expect(submit).toBeEnabled();
  fireEvent.click(submit);

  await waitFor(() => expect(submitCommunityNodeIndexingRequest).toHaveBeenCalledTimes(1));
  expect(submitCommunityNodeIndexingRequest).toHaveBeenCalledWith(
    expect.objectContaining({
      scope_kind: 'private_channel',
      channel_id: 'channel-1',
      confirm_private_channel_secret_disclosure: true,
    })
  );
});
