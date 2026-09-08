import { useState } from 'react';
import type { Meta, StoryObj } from '@storybook/react-vite';
import { ConsentGateView, type ConsentGateViewProps } from './ConsentGateView';
import { useTranslation } from 'react-i18next';
import { normalizeSupportedLocale } from '@/i18n';
import { changeDesktopLocale } from '@/i18n/changeLocale';

const documents = ['terms', 'privacy'].map((slug) => ({
  slug, currentVersion: 5, acceptedVersion: null, acceptedAt: null,
  acceptedLanguage: null, acceptedAppVersion: null, effectiveDate: '2026-09-03',
  authoritativeLanguage: 'ja', materialChange: true,
  controllerName: 'Preview Distributor', contact: 'privacy@example.test',
}));

function InteractiveGate(args: ConsentGateViewProps) {
  const { i18n } = useTranslation();
  const [ageAttested, setAgeAttested] = useState(args.ageAttested);
  const [declined, setDeclined] = useState(args.declined);
  return <ConsentGateView {...args} ageAttested={ageAttested} declined={declined}
    locale={normalizeSupportedLocale(i18n.resolvedLanguage)} onLocaleChange={changeDesktopLocale}
    onAgeAttestedChange={setAgeAttested} onDecline={() => setDeclined(true)} />;
}

const meta = {
  title: 'Startup/ConsentGate', component: ConsentGateView,
  parameters: { layout: 'fullscreen' },
  args: {
    documents, updated: false, attestationRequired: true, ageAttested: false,
    accepting: false, error: null, declined: false,
    locale: 'en', onLocaleChange: () => {},
    onAgeAttestedChange: () => {}, onAccept: () => {}, onDecline: () => {},
  },
  render: (args) => <InteractiveGate {...args} />,
} satisfies Meta<typeof ConsentGateView>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Unchecked: Story = {};
export const Checked: Story = { args: { ageAttested: true } };
export const Pending: Story = { args: { ageAttested: true, accepting: true } };
export const SaveError: Story = { args: { ageAttested: true, error: 'Storage unavailable. Please try again.' } };
export const Declined: Story = { args: { declined: true } };
export const RenewedConsent: Story = { args: { updated: true, attestationRequired: false } };
export const OutdatedAttestation: Story = { args: { updated: true } };
export const LocaleSaveError: Story = { args: { localeSaveFailed: true } };
