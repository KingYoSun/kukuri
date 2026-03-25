import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { CommunityNodePanel } from './CommunityNodePanel';
import { ConnectivityPanel } from './ConnectivityPanel';
import { DiscoveryPanel } from './DiscoveryPanel';
import {
  communityNodePanelFixture,
  connectivityPanelFixture,
  discoveryPanelFixture,
} from './fixtures';
import { SettingsActionRow } from './SettingsActionRow';
import { SettingsDiagnosticList } from './SettingsDiagnosticList';
import { SettingsMetricGrid } from './SettingsMetricGrid';

test('connectivity panel renders loading and topic detail states', async () => {
  const user = userEvent.setup();
  const onImportPeer = vi.fn();

  render(
    <ConnectivityPanel
      view={{
        ...connectivityPanelFixture,
        status: 'loading',
        summaryLabel: 'loading',
      }}
      onPeerTicketInputChange={() => {}}
      onImportPeer={onImportPeer}
    />
  );

  expect(screen.getByText('Loading connectivity diagnostics…')).toBeInTheDocument();
  expect(screen.getByText('Topic Connectivity Detail')).toBeInTheDocument();
  expect(screen.getByText('timed out waiting for gossip topic join')).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Import Peer' }));
  expect(onImportPeer).toHaveBeenCalledTimes(1);
});

test('discovery panel keeps env-locked seed editor read-only', () => {
  render(
    <DiscoveryPanel
      view={{
        ...discoveryPanelFixture,
        envLocked: true,
        seedPeersMessage: 'Environment overrides discovery seeds; editing is disabled.',
      }}
      saveDisabled
      resetDisabled
      onSeedPeersChange={() => {}}
      onSave={() => {}}
      onReset={() => {}}
    />
  );

  expect(screen.getByLabelText('Seed Peers')).toHaveAttribute('readonly');
  expect(
    screen.getByText('Environment overrides discovery seeds; editing is disabled.')
  ).toBeInTheDocument();
});

test('community node panel renders ready and error states', async () => {
  const user = userEvent.setup();
  const onAcceptConsents = vi.fn();

  render(
    <CommunityNodePanel
      view={{
        ...communityNodePanelFixture,
        panelError: 'failed to update community nodes',
      }}
      saveDisabled={false}
      resetDisabled={false}
      clearDisabled={false}
      onBaseUrlsChange={() => {}}
      onSaveNodes={() => {}}
      onReset={() => {}}
      onClearNodes={() => {}}
      onAuthenticate={() => {}}
      onFetchConsents={() => {}}
      onAcceptConsents={onAcceptConsents}
      onRefresh={() => {}}
      onClearToken={() => {}}
    />
  );

  expect(screen.getByText('failed to update community nodes')).toBeInTheDocument();
  expect(screen.getByRole('heading', { name: 'https://api.kukuri.app' })).toBeInTheDocument();

  await user.click(screen.getAllByRole('button', { name: 'Accept' })[0]);
  expect(onAcceptConsents).toHaveBeenCalledWith('https://api.kukuri.app');
});

test('settings panels avoid the legacy grid classname collision', () => {
  const { container } = render(
    <div>
      <ConnectivityPanel
        view={connectivityPanelFixture}
        onPeerTicketInputChange={() => {}}
        onImportPeer={() => {}}
      />
      <DiscoveryPanel
        view={discoveryPanelFixture}
        saveDisabled={false}
        resetDisabled={false}
        onSeedPeersChange={() => {}}
        onSave={() => {}}
        onReset={() => {}}
      />
      <CommunityNodePanel
        view={communityNodePanelFixture}
        saveDisabled={false}
        resetDisabled={false}
        clearDisabled={false}
        onBaseUrlsChange={() => {}}
        onSaveNodes={() => {}}
        onReset={() => {}}
        onClearNodes={() => {}}
        onAuthenticate={() => {}}
        onFetchConsents={() => {}}
        onAcceptConsents={() => {}}
        onRefresh={() => {}}
        onClearToken={() => {}}
      />
      <SettingsMetricGrid items={connectivityPanelFixture.metrics} />
      <SettingsDiagnosticList items={discoveryPanelFixture.diagnostics} columns={2} />
      <SettingsActionRow>
        <button type='button'>Action</button>
      </SettingsActionRow>
    </div>
  );

  expect(container.querySelector('.grid')).toBeNull();
});
