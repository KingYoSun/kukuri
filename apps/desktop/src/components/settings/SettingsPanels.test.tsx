import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, test, vi } from 'vitest';

import { AppearancePanel } from './AppearancePanel';
import { CommunityNodePanel } from './CommunityNodePanel';
import { ConnectivityPanel } from './ConnectivityPanel';
import { DiscoveryPanel } from './DiscoveryPanel';
import { ReactionsPanel } from './ReactionsPanel';
import {
  createAppearancePanelFixture,
  createCommunityNodePanelFixture,
  createConnectivityPanelFixture,
  createDiscoveryPanelFixture,
} from './fixtures';
import { SettingsActionRow } from './SettingsActionRow';
import { SettingsDiagnosticList } from './SettingsDiagnosticList';
import { SettingsMetricGrid } from './SettingsMetricGrid';

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function installCropperMocks() {
  vi.spyOn(URL, 'createObjectURL').mockImplementation(() => 'blob:crop-preview');
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});
  vi.stubGlobal(
    'Image',
    class {
      naturalWidth = 320;
      naturalHeight = 240;
      onload: null | (() => void) = null;
      onerror: null | (() => void) = null;

      set src(_value: string) {
        queueMicrotask(() => {
          this.onload?.();
        });
      }
    }
  );
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
    drawImage: vi.fn(),
  } as unknown as CanvasRenderingContext2D);
  vi.spyOn(HTMLCanvasElement.prototype, 'toBlob').mockImplementation((callback) => {
    callback(new Blob([Uint8Array.from([1, 2, 3, 4])], { type: 'image/png' }));
  });
}

test('appearance panel switches the selected theme immediately', async () => {
  const user = userEvent.setup();
  const onThemeChange = vi.fn();
  const appearancePanelFixture = createAppearancePanelFixture();

  render(
    <AppearancePanel
      view={appearancePanelFixture}
      onThemeChange={onThemeChange}
      onLocaleChange={() => {}}
    />
  );

  await user.click(screen.getByRole('radio', { name: /Light/i }));
  expect(onThemeChange).toHaveBeenCalledWith('light');
  expect(screen.getByRole('radio', { name: /Dark/i })).toHaveAttribute('aria-checked', 'true');
});

test('appearance panel removes redundant explanatory copy and duplicate section titles', () => {
  const appearancePanelFixture = createAppearancePanelFixture();

  render(
    <AppearancePanel
      view={appearancePanelFixture}
      onThemeChange={() => {}}
      onLocaleChange={() => {}}
    />
  );

  expect(screen.queryByRole('heading', { name: 'Appearance' })).not.toBeInTheDocument();
  expect(screen.queryByText('dark theme selected')).not.toBeInTheDocument();
  expect(
    screen.queryByText('Theme changes apply immediately on this device and stay local to this desktop.')
  ).not.toBeInTheDocument();
  expect(
    screen.queryByText(
      'Language changes apply immediately on this device and stay local to this desktop.'
    )
  ).not.toBeInTheDocument();
  expect(
    screen.queryByText('High-contrast solid surfaces for low-light work.')
  ).not.toBeInTheDocument();
  expect(
    screen.queryByText('Brighter solid surfaces for daytime readability.')
  ).not.toBeInTheDocument();
  expect(screen.getByRole('radiogroup', { name: 'Theme mode' })).toBeInTheDocument();
  expect(screen.getByLabelText('Language')).toBeInTheDocument();
});

test('connectivity panel renders loading and topic detail states', async () => {
  const user = userEvent.setup();
  const onImportPeer = vi.fn();
  const connectivityPanelFixture = createConnectivityPanelFixture();

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
  const discoveryPanelFixture = createDiscoveryPanelFixture();
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
  const communityNodePanelFixture = createCommunityNodePanelFixture();

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
  const appearancePanelFixture = createAppearancePanelFixture();
  const connectivityPanelFixture = createConnectivityPanelFixture();
  const discoveryPanelFixture = createDiscoveryPanelFixture();
  const communityNodePanelFixture = createCommunityNodePanelFixture();

  const { container } = render(
    <div>
      <AppearancePanel
        view={appearancePanelFixture}
        onThemeChange={() => {}}
        onLocaleChange={() => {}}
      />
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

test('reactions panel renders owned and saved assets and removes bookmarks', async () => {
  const user = userEvent.setup();
  const onRemoveBookmark = vi.fn();

  render(
    <ReactionsPanel
      view={{
        status: 'ready',
        summaryLabel: 'ready',
        ownedAssets: [
          {
            asset_id: 'asset-owned',
            owner_pubkey: 'a'.repeat(64),
            blob_hash: 'blob-owned',
            search_key: 'party-parrot',
            mime: 'image/png',
            bytes: 128,
            width: 128,
            height: 128,
          },
        ],
        bookmarkedAssets: [
          {
            asset_id: 'asset-saved',
            owner_pubkey: 'b'.repeat(64),
            blob_hash: 'blob-saved',
            search_key: 'saved-cat',
            mime: 'image/gif',
            bytes: 256,
            width: 128,
            height: 128,
          },
        ],
      }}
      creating={false}
      mediaObjectUrls={{
        'blob-owned': 'https://example.com/owned.png',
        'blob-saved': 'https://example.com/saved.gif',
      }}
      onCreateAsset={() => {}}
      onRemoveBookmark={onRemoveBookmark}
    />
  );

  expect(screen.getByText('My custom reactions')).toBeInTheDocument();
  expect(screen.getByText('party-parrot')).toBeInTheDocument();
  expect(screen.getByText('saved-cat')).toBeInTheDocument();
  expect(screen.getByAltText('asset-owned')).toHaveAttribute('src', 'https://example.com/owned.png');
  expect(screen.getByAltText('asset-saved')).toHaveAttribute('src', 'https://example.com/saved.gif');

  await user.click(screen.getByRole('button', { name: 'Clear' }));
  expect(onRemoveBookmark).toHaveBeenCalledWith('asset-saved');
});

test('connectivity panel shows the summary state only once in the sync status card', () => {
  const connectivityPanelFixture = createConnectivityPanelFixture();

  render(
    <ConnectivityPanel
      view={{
        ...connectivityPanelFixture,
        summaryLabel: 'waiting',
      }}
      onPeerTicketInputChange={() => {}}
      onImportPeer={() => {}}
    />
  );

  expect(screen.getAllByText('waiting')).toHaveLength(1);
});

test('reactions panel crops an uploaded image before creating a custom asset', async () => {
  installCropperMocks();
  const user = userEvent.setup();
  const onCreateAsset = vi.fn();

  render(
    <ReactionsPanel
      view={{
        status: 'ready',
        summaryLabel: 'ready',
        ownedAssets: [],
        bookmarkedAssets: [],
      }}
      creating={false}
      onCreateAsset={onCreateAsset}
      onRemoveBookmark={() => {}}
    />
  );

  await user.upload(
    screen.getByLabelText('Upload image'),
    new File([Uint8Array.from([9, 8, 7, 6])], 'party.png', { type: 'image/png' })
  );

  const cropDialog = await screen.findByRole('dialog', { name: 'Crop reaction image' });
  expect(within(cropDialog).getByRole('slider')).toBeInTheDocument();

  await user.click(within(cropDialog).getByRole('button', { name: 'Save' }));

  await waitFor(() => {
    expect(screen.queryByRole('dialog', { name: 'Crop reaction image' })).not.toBeInTheDocument();
  });

  await user.type(screen.getByLabelText('Search key'), 'party');
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);

  expect(onCreateAsset).toHaveBeenCalledWith(
    expect.objectContaining({ name: 'party.png' }),
    expect.objectContaining({
      size: expect.any(Number),
      x: expect.any(Number),
      y: expect.any(Number),
    }),
    'party'
  );
});
