import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import type { DesktopApi } from '@/lib/api';
import { InvokeError } from '@/lib/api/invoke/error';

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
  expect(screen.getByRole('checkbox')).not.toBeChecked();
  expect(submit).toBeDisabled();
});

test('private confirmation and status reset when the selected node changes', async () => {
  const submitCommunityNodeIndexingRequest = vi.fn().mockResolvedValue({
    request_id: 'request-3',
    status: 'pending',
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
      eligibleNodeBaseUrls={['https://index-a.example', 'https://index-b.example']}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );

  const confirmation = screen.getByRole('checkbox');
  const submit = screen.getByRole('button', { name: 'Submit request' });
  fireEvent.click(confirmation);
  fireEvent.click(submit);

  expect(await screen.findByText('The request is pending review.')).toBeInTheDocument();
  expect(confirmation).not.toBeChecked();
  expect(submit).toBeDisabled();

  fireEvent.click(confirmation);
  fireEvent.change(screen.getByLabelText('Community Node'), {
    target: { value: 'https://index-b.example' },
  });

  expect(confirmation).not.toBeChecked();
  expect(submit).toBeDisabled();
  expect(screen.queryByText('The request is pending review.')).not.toBeInTheDocument();

  fireEvent.click(confirmation);
  fireEvent.click(submit);
  await waitFor(() => expect(submitCommunityNodeIndexingRequest).toHaveBeenCalledTimes(2));
  expect(submitCommunityNodeIndexingRequest).toHaveBeenLastCalledWith(
    expect.objectContaining({ base_url: 'https://index-b.example' })
  );
});

test('failed private request consumes confirmation and node change clears the error', async () => {
  const submitCommunityNodeIndexingRequest = vi
    .fn()
    .mockRejectedValueOnce(new Error('request failed'))
    .mockResolvedValueOnce({ request_id: 'request-4', status: 'approved' });
  render(
    <CommunityIndexingRequestDialog
      api={{ submitCommunityNodeIndexingRequest } as unknown as DesktopApi}
      target={{
        kind: 'private_channel',
        topicId: 'kukuri:topic:demo',
        channelId: 'channel-1',
        channelLabel: 'Core',
      }}
      eligibleNodeBaseUrls={['https://index-a.example', 'https://index-b.example']}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );

  const confirmation = screen.getByRole('checkbox');
  const submit = screen.getByRole('button', { name: 'Submit request' });
  fireEvent.click(confirmation);
  fireEvent.click(submit);

  expect(await screen.findByText('The indexing request failed. Please try again later.')).toBeInTheDocument();
  expect(confirmation).not.toBeChecked();
  expect(submit).toBeDisabled();

  fireEvent.click(confirmation);
  fireEvent.change(screen.getByLabelText('Community Node'), {
    target: { value: 'https://index-b.example' },
  });
  expect(confirmation).not.toBeChecked();
  expect(submit).toBeDisabled();
  expect(
    screen.queryByText('The indexing request failed. Please try again later.')
  ).not.toBeInTheDocument();
});

test('private confirmation and status reset when the request target changes', async () => {
  const submitCommunityNodeIndexingRequest = vi.fn().mockResolvedValue({
    request_id: 'request-5',
    status: 'approved',
  });
  const eligibleNodeBaseUrls = ['https://index.example'];
  const props = {
    api: { submitCommunityNodeIndexingRequest } as unknown as DesktopApi,
    eligibleNodeBaseUrls,
    onOpenChange: vi.fn(),
    onOpenCommunityNodeSettings: vi.fn(),
  };
  const { rerender } = render(
    <CommunityIndexingRequestDialog
      {...props}
      target={{
        kind: 'private_channel',
        topicId: 'kukuri:topic:demo',
        channelId: 'channel-1',
        channelLabel: 'Core',
      }}
    />
  );

  const confirmation = screen.getByRole('checkbox');
  fireEvent.click(confirmation);
  fireEvent.click(screen.getByRole('button', { name: 'Submit request' }));
  expect(await screen.findByText('Indexing is approved.')).toBeInTheDocument();
  fireEvent.click(confirmation);

  rerender(
    <CommunityIndexingRequestDialog
      {...props}
      target={{
        kind: 'private_channel',
        topicId: 'kukuri:topic:demo',
        channelId: 'channel-2',
        channelLabel: 'Review',
      }}
    />
  );

  expect(screen.getByRole('checkbox')).not.toBeChecked();
  expect(screen.getByRole('button', { name: 'Submit request' })).toBeDisabled();
  expect(screen.queryByText('Indexing is approved.')).not.toBeInTheDocument();
});

test('stale private request result does not overwrite a changed target', async () => {
  let resolveRequest:
    | ((value: { request_id: string; status: 'approved' }) => void)
    | undefined;
  const response = new Promise<{ request_id: string; status: 'approved' }>((resolve) => {
    resolveRequest = resolve;
  });
  const submitCommunityNodeIndexingRequest = vi.fn().mockReturnValue(response);
  const eligibleNodeBaseUrls = ['https://index.example'];
  const props = {
    api: { submitCommunityNodeIndexingRequest } as unknown as DesktopApi,
    eligibleNodeBaseUrls,
    onOpenChange: vi.fn(),
    onOpenCommunityNodeSettings: vi.fn(),
  };
  const { rerender } = render(
    <CommunityIndexingRequestDialog
      {...props}
      target={{
        kind: 'private_channel',
        topicId: 'kukuri:topic:demo',
        channelId: 'channel-1',
        channelLabel: 'Core',
      }}
    />
  );

  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Submit request' }));
  rerender(
    <CommunityIndexingRequestDialog
      {...props}
      target={{
        kind: 'private_channel',
        topicId: 'kukuri:topic:demo',
        channelId: 'channel-2',
        channelLabel: 'Review',
      }}
    />
  );

  await act(async () => {
    resolveRequest?.({ request_id: 'request-6', status: 'approved' });
    await response;
  });

  expect(screen.getByText('Target: Review')).toBeInTheDocument();
  expect(screen.queryByText('Indexing is approved.')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Submit request' })).toBeDisabled();
});

// #698: 適格一覧の参照が変わるだけ(内容不変)では確認状態を消さない。
test('an equal eligible list rendered as a new array keeps the private confirmation', async () => {
  const submitCommunityNodeIndexingRequest = vi.fn().mockResolvedValue({
    request_id: 'request-1',
    status: 'pending',
  });
  const api = { submitCommunityNodeIndexingRequest } as unknown as DesktopApi;
  const target = { kind: 'private_channel' as const, topicId: 'kukuri:topic:demo', channelId: 'ch-1', channelLabel: 'demo' };
  const { rerender } = render(
    <CommunityIndexingRequestDialog
      api={api}
      target={target}
      eligibleNodeBaseUrls={['https://index.example']}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );
  fireEvent.click(screen.getByRole('checkbox'));
  expect(screen.getByRole('checkbox')).toBeChecked();

  rerender(
    <CommunityIndexingRequestDialog
      api={api}
      target={target}
      eligibleNodeBaseUrls={['https://index.example']}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );
  expect(screen.getByRole('checkbox')).toBeChecked();
  expect(screen.getByRole('button', { name: 'Submit request' })).toBeEnabled();
});

// #698: 選択ノードが適格一覧から外れると、確認済みでも申請(秘密値)を送らない。
test('a selected node dropped from the eligible list cannot receive a private request', async () => {
  const submitCommunityNodeIndexingRequest = vi.fn().mockResolvedValue({
    request_id: 'request-1',
    status: 'pending',
  });
  const api = { submitCommunityNodeIndexingRequest } as unknown as DesktopApi;
  const target = { kind: 'private_channel' as const, topicId: 'kukuri:topic:demo', channelId: 'ch-1', channelLabel: 'demo' };
  const { rerender } = render(
    <CommunityIndexingRequestDialog
      api={api}
      target={target}
      eligibleNodeBaseUrls={['https://index-a.example', 'https://index-b.example']}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );
  fireEvent.click(screen.getByRole('checkbox'));
  expect(screen.getByRole('button', { name: 'Submit request' })).toBeEnabled();

  // A の同意/能力が失効し、適格一覧が [B] だけになる。
  rerender(
    <CommunityIndexingRequestDialog
      api={api}
      target={target}
      eligibleNodeBaseUrls={['https://index-b.example']}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );
  // 内容が変わったので確認は消え、送信は無効。改めて確認しても送信先は B になる。
  expect(screen.getByRole('checkbox')).not.toBeChecked();
  expect(screen.getByRole('button', { name: 'Submit request' })).toBeDisabled();
  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Submit request' }));
  await waitFor(() => expect(submitCommunityNodeIndexingRequest).toHaveBeenCalledTimes(1));
  expect(submitCommunityNodeIndexingRequest).toHaveBeenCalledWith(
    expect.objectContaining({ base_url: 'https://index-b.example' })
  );
  expect(submitCommunityNodeIndexingRequest).not.toHaveBeenCalledWith(
    expect.objectContaining({ base_url: 'https://index-a.example' })
  );
});

test('explains the server-side indexing request gate with stable codes', async () => {
  // #713: 索引未提供・有効化失効のノードは申請を受け付けない。安定コードで案内する。
  const submitCommunityNodeIndexingRequest = vi
    .fn()
    .mockRejectedValueOnce(new InvokeError('INDEXING_REQUEST_NOT_CONFIGURED', 'gate'))
    .mockRejectedValueOnce(new InvokeError('INDEXING_REQUEST_NOT_ACTIVATED', 'gate'));
  render(
    <CommunityIndexingRequestDialog
      api={{ submitCommunityNodeIndexingRequest } as unknown as DesktopApi}
      target={{ kind: 'public_topic', topicId: 'kukuri:topic:demo' }}
      eligibleNodeBaseUrls={['https://index-a.example']}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );

  const submit = screen.getByRole('button', { name: 'Submit request' });
  fireEvent.click(submit);
  expect(
    await screen.findByText(
      'This Community Node does not provide indexing, so it does not accept requests.'
    )
  ).toBeInTheDocument();

  fireEvent.click(submit);
  expect(
    await screen.findByText(
      'Indexing on this Community Node is temporarily unavailable, so it does not accept requests.'
    )
  ).toBeInTheDocument();
});
