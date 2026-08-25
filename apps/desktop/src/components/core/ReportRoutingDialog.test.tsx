import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import { type ReportRoutingPlan } from '@/lib/api/reportRouting';
import { InvokeError } from '@/lib/api/invoke/error';

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
        rightsRequestUrl: 'https://index.example/rights-requests/new',
        rightsRequestPolicyUrl: 'https://index.example/rights-infringement-policy',
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

test('routes rights claims to the dedicated scope-first intake without posting a general report', () => {
  const onSubmit = vi.fn();
  render(
    <ReportRoutingDialog
      open
      onOpenChange={vi.fn()}
      subject={subject}
      plan={endpointPlan}
      onSubmit={onSubmit}
    />,
  );

  fireEvent.change(screen.getByLabelText('Reason'), {
    target: { value: 'rights_infringement' },
  });
  const link = screen.getByRole('link', { name: /review scope and submit a rights request/i });
  expect(link).toHaveAttribute('href', 'https://index.example/rights-requests/new');
  expect(screen.getByText(/first review what this Community Node can and cannot do/i)).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Send report' })).not.toBeInTheDocument();
  expect(onSubmit).not.toHaveBeenCalled();
});

test('keeps local actions available when manifest resolution fails', () => {
  render(
    <ReportRoutingDialog
      open
      onOpenChange={vi.fn()}
      subject={subject}
      plan={unknownPlan}
      onSubmit={vi.fn()}
      resolveError='unavailable'
      localActions={<button type='button'>Mute author</button>}
    />,
  );

  expect(
    screen.getByText('Could not refresh report targets. No default destination will be used.'),
  ).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Mute author' })).toBeInTheDocument();
});

test('uses the appeal presentation, omits contact, and confirms the disputed signal', async () => {
  const onSubmitted = vi.fn();
  const onSubmit = vi.fn().mockResolvedValue({
    status: 'submitted',
    reference_id: 'report-1',
    disputed_risk_signal_id: 'signal-1',
  });
  render(
    <ReportRoutingDialog
      open
      onOpenChange={vi.fn()}
      subject={{ kind: 'profile', id: 'author-pubkey' }}
      plan={endpointPlan}
      appeal={{ riskSignalId: 'signal-1', issuerNodeId: 'issuer-node' }}
      onSubmit={onSubmit}
      onSubmitted={onSubmitted}
    />,
  );

  expect(screen.getByText('Appeal a risk assessment')).toBeInTheDocument();
  expect(screen.queryByLabelText(/contact|連絡先/i)).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Submit appeal' }));

  await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
  expect(onSubmit).toHaveBeenCalledWith(
    expect.objectContaining({
      reason: 'other',
      reporterContact: '',
      appeal: { risk_signal_id: 'signal-1' },
    }),
  );
  expect(await screen.findByText(/Appealed assessment: signal-1/)).toBeInTheDocument();
  expect(onSubmitted).toHaveBeenCalledTimes(1);
});

test('shows a stable localized message for an invalid appeal', async () => {
  render(
    <ReportRoutingDialog
      open
      onOpenChange={vi.fn()}
      subject={{ kind: 'profile', id: 'author-pubkey' }}
      plan={endpointPlan}
      appeal={{ riskSignalId: 'signal-1', issuerNodeId: 'issuer-node' }}
      onSubmit={vi.fn().mockRejectedValue(new InvokeError('INVALID_APPEAL', 'unknown'))}
    />,
  );

  fireEvent.click(screen.getByRole('button', { name: 'Submit appeal' }));
  expect(
    await screen.findByText(/The assessment could not be verified or has already been resolved/),
  ).toBeInTheDocument();
  expect(screen.queryByText('unknown')).not.toBeInTheDocument();
});

// #696: 最新 manifest の取得中は送信も連絡先の複写もしない。
test('disables sending and contact copy while the latest report targets are resolving', () => {
  const onSubmit = vi.fn();
  const onCopyContact = vi.fn();
  const contactPlan: ReportRoutingPlan = {
    ...endpointPlan,
    candidates: [
      {
        target: { ...endpointPlan.candidates[0].target, reportEndpoint: undefined },
        contact: { kind: 'contact', value: 'abuse@index.example' },
      },
    ],
  };
  const { rerender } = render(
    <ReportRoutingDialog
      open
      onOpenChange={vi.fn()}
      subject={subject}
      plan={endpointPlan}
      onSubmit={onSubmit}
      resolving
    />,
  );
  expect(screen.getByText(/Checking the latest report targets/)).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Send report' })).toBeDisabled();
  fireEvent.click(screen.getByRole('button', { name: 'Send report' }));
  expect(onSubmit).not.toHaveBeenCalled();

  rerender(
    <ReportRoutingDialog
      open
      onOpenChange={vi.fn()}
      subject={subject}
      plan={contactPlan}
      onSubmit={onSubmit}
      onCopyContact={onCopyContact}
      resolving
    />,
  );
  const copyButton = screen.getByRole('button', { name: 'Send report' });
  expect(copyButton).toHaveTextContent('Copy abuse contact');
  expect(copyButton).toBeDisabled();
  fireEvent.click(copyButton);
  expect(onCopyContact).not.toHaveBeenCalled();
});

// #696: 候補が後から届いても、入力途中の詳細と選択候補は保たれる。
test('keeps typed details and the selected candidate when the candidate list is refreshed', () => {
  const { rerender } = render(
    <ReportRoutingDialog
      open
      onOpenChange={vi.fn()}
      subject={subject}
      plan={endpointPlan}
      onSubmit={vi.fn()}
    />,
  );
  const details = screen.getByPlaceholderText(/Describe the problem/);
  fireEvent.change(details, { target: { value: 'typed while refreshing' } });

  const refreshedPlan: ReportRoutingPlan = {
    ...endpointPlan,
    candidates: [
      {
        target: {
          nodeBaseUrl: 'https://media.example',
          capability: 'media_cache',
          reportEndpoint: 'https://media.example/v1/report',
          authorityScope: ['this_node'],
        },
        contact: { kind: 'endpoint', value: 'https://media.example/v1/report' },
      },
      { ...endpointPlan.candidates[0] },
    ],
  };
  rerender(
    <ReportRoutingDialog
      open
      onOpenChange={vi.fn()}
      subject={subject}
      plan={refreshedPlan}
      onSubmit={vi.fn()}
    />,
  );
  expect(screen.getByDisplayValue('typed while refreshing')).toBeInTheDocument();
  expect(screen.getByRole('radio', { name: /index\.example/ })).toBeChecked();
});
