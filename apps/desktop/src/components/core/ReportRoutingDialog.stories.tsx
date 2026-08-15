import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { type ReportRoutingPlan } from '@/lib/api/reportRouting';

import { Button } from '@/components/ui/button';

import { ReportRoutingDialog, type ReportRoutingSubject } from './ReportRoutingDialog';

const meta = {
  title: 'Core/ReportRoutingDialog',
  parameters: {
    layout: 'centered',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

const subject: ReportRoutingSubject = {
  kind: 'post',
  id: 'post-1',
  label: 'alice',
};

const endpointPlan: ReportRoutingPlan = {
  provenanceUnknown: false,
  observedButUnresolved: false,
  localActionsOnly: false,
  candidates: [
    {
      target: {
        nodeBaseUrl: 'https://index.kukuri.example',
        nodeId: 'index-node',
        capability: 'community_index',
        reportEndpoint: 'https://index.kukuri.example/v1/report',
        abuseContact: 'abuse@index.kukuri.example',
        policyUrl: 'https://index.kukuri.example/moderation',
        authorityScope: ['this_node'],
      },
      contact: { kind: 'endpoint', value: 'https://index.kukuri.example/v1/report' },
    },
    {
      target: {
        nodeBaseUrl: 'https://cache.kukuri.example',
        capability: 'media_cache',
        abuseContact: 'abuse@cache.kukuri.example',
        authorityScope: ['this_node'],
      },
      contact: { kind: 'contact', value: 'abuse@cache.kukuri.example' },
    },
  ],
};

const unknownPlan: ReportRoutingPlan = {
  provenanceUnknown: true,
  observedButUnresolved: false,
  localActionsOnly: true,
  candidates: [],
};

const observedUnresolvedPlan: ReportRoutingPlan = {
  provenanceUnknown: false,
  observedButUnresolved: true,
  localActionsOnly: true,
  candidates: [],
};

function DialogHarness({
  plan,
  label,
  appeal,
}: {
  plan: ReportRoutingPlan;
  label: string;
  appeal?: { riskSignalId: string; issuerNodeId: string };
}) {
  const [open, setOpen] = useState(false);
  return (
    <div className='flex flex-col items-center gap-4'>
      <Button type='button' onClick={() => setOpen(true)}>
        {label}
      </Button>
      <ReportRoutingDialog
        open={open}
        onOpenChange={setOpen}
        subject={subject}
        plan={plan}
        appeal={appeal}
        onSubmit={async ({ candidate, reason, appeal: appealInput }) => ({
          status: 'submitted',
          reference_id: `story-${candidate.target.capability}-${reason}`,
          disputed_risk_signal_id: appealInput?.risk_signal_id ?? null,
        })}
        onCopyContact={() => {}}
        localActions={
          <Button type='button' variant='secondary'>
            Mute author
          </Button>
        }
      />
    </div>
  );
}

export const ResolvedTargets: Story = {
  render: () => <DialogHarness plan={endpointPlan} label='Report (resolved targets)' />,
};

export const UnknownProvenance: Story = {
  render: () => <DialogHarness plan={unknownPlan} label='Report (unknown provenance)' />,
};

export const ObservedButUnresolved: Story = {
  render: () => <DialogHarness plan={observedUnresolvedPlan} label='Report (observed, unresolved)' />,
};

const appealPlan: ReportRoutingPlan = {
  provenanceUnknown: false,
  observedButUnresolved: false,
  localActionsOnly: false,
  candidates: [
    {
      target: {
        nodeBaseUrl: 'https://index.kukuri.example',
        nodeId: 'index-node',
        capability: 'trust_signal',
        reportEndpoint: 'https://index.kukuri.example/v1/report',
        policyUrl: 'https://index.kukuri.example/moderation',
        authorityScope: ['this_node'],
      },
      contact: { kind: 'endpoint', value: 'https://index.kukuri.example/v1/report' },
    },
  ],
};

export const Appeal: Story = {
  render: () => (
    <DialogHarness
      plan={appealPlan}
      label='異議申し立てを開く'
      appeal={{ riskSignalId: 'signal-1', issuerNodeId: 'index-node' }}
    />
  ),
};
