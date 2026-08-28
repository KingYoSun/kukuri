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

test('shows the settings fallback when no eligible node accepts feedback', () => {
  const onOpenCommunityNodeSettings = vi.fn();
  render(
    <TesterFeedbackDialog
      api={{} as unknown as DesktopApi}
      open
      eligibleNodeBaseUrls={[]}
      onOpenChange={vi.fn()}
      onOpenCommunityNodeSettings={onOpenCommunityNodeSettings}
    />
  );

  expect(
    screen.getByText('No authenticated and consented Community Node currently accepts tester feedback.')
  ).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled();
  fireEvent.click(screen.getByRole('button', { name: 'Open Community Node settings' }));
  expect(onOpenCommunityNodeSettings).toHaveBeenCalledTimes(1);
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
