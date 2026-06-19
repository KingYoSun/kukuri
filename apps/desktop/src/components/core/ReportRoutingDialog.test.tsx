import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import { type ReportRoutingPlan } from '@/lib/api/reportRouting';

import { ReportRoutingDialog, type ReportRoutingSubject } from './ReportRoutingDialog';

const subject: ReportRoutingSubject = { kind: 'post', id: 'post-1', label: 'alice' };

const endpointPlan: ReportRoutingPlan = {
  provenanceUnknown: false,
  observedButUnresolved: false,
  localActionsOnly: false,
  candidates: [
    {
      target: {
        nodeBaseUrl: 'https://index.example',
        capability: 'community_index',
        reportEndpoint: 'https://index.example/v1/report',
        abuseContact: 'abuse@index.example',
        authorityScope: ['this_node'],
      },
      contact: { kind: 'endpoint', value: 'https://index.example/v1/report' },
    },
  ],
};

const unknownPlan: ReportRoutingPlan = {
  provenanceUnknown: true,
  observedButUnresolved: false,
  localActionsOnly: true,
  candidates: [],
};

test('always shows the boundary notice and routes a report to the resolved node', async () => {
  const onSubmit = vi.fn().mockResolvedValue({ status: 'submitted', reference_id: 'ref-1' });
  render(
    <ReportRoutingDialog
      open
      onOpenChange={vi.fn()}
      subject={subject}
      plan={endpointPlan}
      onSubmit={onSubmit}
    />,
  );

  // 送信前に「kukuri 全体ではない」境界を常に明示する。
  expect(
    screen.getByText(/sent to a specific community node, not to the kukuri network/i),
  ).toBeInTheDocument();
  // 通報先 node と capability が表示される。
  expect(screen.getByText('index.example')).toBeInTheDocument();
  expect(screen.getByText('Community index')).toBeInTheDocument();

  fireEvent.click(screen.getByRole('button', { name: 'Send report' }));

  await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
  expect(onSubmit).toHaveBeenCalledWith(
    expect.objectContaining({
      reason: 'spam',
      candidate: expect.objectContaining({
        target: expect.objectContaining({ nodeBaseUrl: 'https://index.example' }),
      }),
    }),
  );
  expect(await screen.findByText(/Reference: ref-1/)).toBeInTheDocument();
});

test('does not offer a default node when provenance is unknown', () => {
  const onSubmit = vi.fn();
  render(
    <ReportRoutingDialog
      open
      onOpenChange={vi.fn()}
      subject={subject}
      plan={unknownPlan}
      onSubmit={onSubmit}
    />,
  );

  expect(screen.getByText('Cannot determine a report target')).toBeInTheDocument();
  // local action のみ案内し、送信ボタンは出さない。
  expect(screen.getByText(/block, mute, or hide this locally/i)).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Send report' })).not.toBeInTheDocument();
  expect(onSubmit).not.toHaveBeenCalled();
});
