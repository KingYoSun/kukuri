import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import type { DesktopApi } from '@/lib/api';
import { InvokeError } from '@/lib/api/invoke/error';

import { TESTER_FEEDBACK_MAX_CHARS, TesterFeedbackDialog } from './TesterFeedbackDialog';

function fillFields() {
  fireEvent.change(screen.getByLabelText('What you were trying to do'), {
    target: { value: 'create a post' },
  });
  fireEvent.change(screen.getByLabelText('What happened'), {
    target: { value: 'nothing happened after pressing send' },
  });
  fireEvent.change(screen.getByLabelText('What felt wrong'), {
    target: { value: 'no error and no success message' },
  });
}

test('sends only the three texts and the selected node, then shows the reference id', async () => {
  const submitCommunityNodeTesterFeedback = vi.fn().mockResolvedValue({
    reference_id: 'feedback-1',
  });
  render(
    <TesterFeedbackDialog
      api={{ submitCommunityNodeTesterFeedback } as unknown as DesktopApi}
      open
      eligibleNodeBaseUrls={['https://node.example']}
      availability={{ state: 'ready', nodes: [] }}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );

  const submit = screen.getByRole('button', { name: 'Send' });
  expect(submit).toBeDisabled();
  fillFields();
  expect(submit).toBeEnabled();
  fireEvent.click(submit);

  await waitFor(() => expect(submitCommunityNodeTesterFeedback).toHaveBeenCalledTimes(1));
  expect(submitCommunityNodeTesterFeedback).toHaveBeenCalledWith({
    base_url: 'https://node.example',
    what_attempted: 'create a post',
    what_happened: 'nothing happened after pressing send',
    what_seemed_wrong: 'no error and no success message',
  });
  expect(await screen.findByText('Feedback sent. Reference ID: feedback-1')).toBeInTheDocument();
});

test('blocks submission when a field exceeds the character limit', () => {
  const submitCommunityNodeTesterFeedback = vi.fn();
  render(
    <TesterFeedbackDialog
      api={{ submitCommunityNodeTesterFeedback } as unknown as DesktopApi}
      open
      eligibleNodeBaseUrls={['https://node.example']}
      availability={{ state: 'ready', nodes: [] }}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );

  fillFields();
  fireEvent.change(screen.getByLabelText('What happened'), {
    target: { value: 'あ'.repeat(TESTER_FEEDBACK_MAX_CHARS + 1) },
  });

  expect(screen.getByText('Each field must be at most 2000 characters.')).toBeInTheDocument();
  const submit = screen.getByRole('button', { name: 'Send' });
  expect(submit).toBeDisabled();
  fireEvent.click(submit);
  expect(submitCommunityNodeTesterFeedback).not.toHaveBeenCalled();
});

test('closes the dialog before opening settings when no eligible node accepts feedback', async () => {
  const onOpenCommunityNodeSettings = vi.fn();
  const onOpenChange = vi.fn();
  const props = {
    api: {} as unknown as DesktopApi,
    eligibleNodeBaseUrls: [],
    availability: { state: 'noNodes' as const, nodes: [] },
    onOpenChange,
    onOpenCommunityNodeSettings,
  };
  const view = render(
    <TesterFeedbackDialog
      {...props}
      open
    />
  );

  expect(
    screen.getByText(/No Community Nodes are configured/)
  ).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled();
  fireEvent.click(screen.getByRole('button', { name: 'Open Community Node settings' }));
  expect(onOpenChange).toHaveBeenCalledWith(false);
  expect(onOpenCommunityNodeSettings).not.toHaveBeenCalled();
  view.rerender(<TesterFeedbackDialog {...props} open={false} />);
  await waitFor(() => expect(onOpenCommunityNodeSettings).toHaveBeenCalledTimes(1));
});

test('maps stable error codes to error notices', async () => {
  const submitCommunityNodeTesterFeedback = vi
    .fn()
    .mockRejectedValue(
      new InvokeError('TESTER_FEEDBACK_NOT_CONFIGURED', 'not accepted', 404)
    );
  render(
    <TesterFeedbackDialog
      api={{ submitCommunityNodeTesterFeedback } as unknown as DesktopApi}
      open
      eligibleNodeBaseUrls={['https://node.example']}
      availability={{ state: 'ready', nodes: [] }}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );

  fillFields();
  fireEvent.click(screen.getByRole('button', { name: 'Send' }));

  expect(
    await screen.findByText('This Community Node does not accept tester feedback.')
  ).toBeInTheDocument();
});

test('keeps draft text on availability updates, blocks a removed destination, and clears it on reopen', () => {
  const submitCommunityNodeTesterFeedback = vi.fn();
  const props = { api: { submitCommunityNodeTesterFeedback } as unknown as DesktopApi,
    open: true, eligibleNodeBaseUrls: ['https://node.example'],
    availability: { state: 'ready' as const, nodes: [] },
    onOpenChange: vi.fn(), onOpenCommunityNodeSettings: vi.fn(),
  };
  const view = render(<TesterFeedbackDialog {...props} />);
  fillFields();
  view.rerender(<TesterFeedbackDialog {...props} eligibleNodeBaseUrls={[]}
    availability={{ state: 'unavailable', nodes: [{ baseUrl: 'https://node.example', label: 'My node', reason: 'connectionFailed' }] }} />);
  expect(screen.getByLabelText('What happened')).toHaveValue('nothing happened after pressing send');
  expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled();
  expect(screen.getByText(/Closing this dialog clears the text/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  expect(submitCommunityNodeTesterFeedback).not.toHaveBeenCalled();
  view.rerender(<TesterFeedbackDialog {...props} />);
  expect(screen.getByLabelText('What happened')).toHaveValue('nothing happened after pressing send');
  expect(screen.getByRole('button', { name: 'Send' })).toBeEnabled();
  view.rerender(<TesterFeedbackDialog {...props} open={false} />);
  view.rerender(<TesterFeedbackDialog {...props} />);
  expect(screen.getByLabelText('What happened')).toHaveValue('');
  expect(submitCommunityNodeTesterFeedback).not.toHaveBeenCalled();
});

test('labels retained destinations as last known when refresh fails without changing the submission guard', () => {
  const submit = vi.fn();
  render(<TesterFeedbackDialog api={{ submitCommunityNodeTesterFeedback: submit } as unknown as DesktopApi}
    open eligibleNodeBaseUrls={['https://node.example']} availability={{ state: 'statusUnavailable', nodes: [] }}
    onOpenChange={vi.fn()} onOpenCommunityNodeSettings={vi.fn()} />);
  expect(screen.getByText(/Showing the last known destinations/)).toBeInTheDocument();
  expect(screen.getByRole('combobox')).toHaveValue('https://node.example');
  expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled();
  fillFields();
  expect(screen.getByRole('button', { name: 'Send' })).toBeEnabled();
  expect(submit).not.toHaveBeenCalled();
});

test.each([
  ['AUTH_REQUIRED', 401, 'Authentication is required for the selected Community Node.'],
  ['CONSENT_REQUIRED', 403, 'Required consent is missing for the selected Community Node.'],
  ['TESTER_FEEDBACK_TRANSPORT_FAILED', undefined, 'Sending feedback failed. Please try again later.'],
] as const)('retains the existing %s notice without automatic resubmission', async (code, status, message) => {
  const submitCommunityNodeTesterFeedback = vi.fn().mockRejectedValue(new InvokeError(code, 'private diagnostic', status));
  render(<TesterFeedbackDialog api={{ submitCommunityNodeTesterFeedback } as unknown as DesktopApi}
    open eligibleNodeBaseUrls={['https://node.example']} availability={{ state: 'ready', nodes: [] }}
    onOpenChange={vi.fn()} onOpenCommunityNodeSettings={vi.fn()} />);
  fillFields();
  fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  expect(await screen.findByText(message)).toBeInTheDocument();
  await waitFor(() => expect(screen.getByRole('button', { name: 'Send' })).toBeEnabled());
  expect(screen.queryByText('private diagnostic')).not.toBeInTheDocument();
  expect(submitCommunityNodeTesterFeedback).toHaveBeenCalledTimes(1);
  expect(screen.getByLabelText('What happened')).toHaveValue('nothing happened after pressing send');
});
