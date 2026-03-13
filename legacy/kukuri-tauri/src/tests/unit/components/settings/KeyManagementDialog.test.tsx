import { describe, it, expect, vi, beforeAll, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { KeyManagementDialog } from '@/components/settings/KeyManagementDialog';
import { useAuthStore } from '@/stores/authStore';
import { useKeyManagementStore } from '@/stores/keyManagementStore';
import { TauriApi } from '@/lib/api/tauri';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';

vi.mock('@/stores/authStore');
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    exportPrivateKey: vi.fn(),
  },
}));
vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));
vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
    info: vi.fn(),
  },
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  save: vi.fn(),
}));
vi.mock('@tauri-apps/plugin-fs', () => ({
  readTextFile: vi.fn(),
  writeTextFile: vi.fn(),
}));

const mockUseAuthStore = useAuthStore as unknown as vi.Mock;
const mockExportPrivateKey = TauriApi.exportPrivateKey as unknown as vi.Mock;
const mockToast = toast as unknown as { success: vi.Mock; error: vi.Mock };
const mockErrorHandler = errorHandler as unknown as { log: vi.Mock; info: vi.Mock };
interface MockAuthStoreState {
  currentUser: { npub: string } | null;
  loginWithNsec: vi.Mock;
}

let mockDialogOpen: ReturnType<typeof vi.fn>;
let mockDialogSave: ReturnType<typeof vi.fn>;
let mockReadTextFile: ReturnType<typeof vi.fn>;
let mockWriteTextFile: ReturnType<typeof vi.fn>;

const renderDialog = () => {
  render(<KeyManagementDialog open onOpenChange={() => undefined} />);
};

const setupAuthStore = (overrides: Partial<MockAuthStoreState> = {}) => {
  const state: MockAuthStoreState = {
    currentUser: null,
    loginWithNsec: vi.fn(),
    ...overrides,
  };
  mockUseAuthStore.mockImplementation(
    (selector: ((state: MockAuthStoreState) => unknown) | undefined) => {
      if (typeof selector === 'function') {
        return selector(state);
      }
      return state;
    },
  );
  return state;
};

beforeAll(async () => {
  const dialogModule = await import('@tauri-apps/plugin-dialog');
  const fsModule = await import('@tauri-apps/plugin-fs');
  mockDialogOpen = dialogModule.open as unknown as ReturnType<typeof vi.fn>;
  mockDialogSave = dialogModule.save as unknown as ReturnType<typeof vi.fn>;
  mockReadTextFile = fsModule.readTextFile as unknown as ReturnType<typeof vi.fn>;
  mockWriteTextFile = fsModule.writeTextFile as unknown as ReturnType<typeof vi.fn>;

  Object.defineProperty(navigator, 'clipboard', {
    value: {
      writeText: vi.fn(),
    },
    configurable: true,
  });
});

beforeEach(() => {
  vi.clearAllMocks();
  useKeyManagementStore.setState((state) => ({
    ...state,
    history: [],
    lastExportedAt: null,
    lastImportedAt: null,
  }));
});

describe('KeyManagementDialog', () => {
  it('exports the private key when requested', async () => {
    const user = userEvent.setup();
    setupAuthStore({
      currentUser: { npub: 'npub1example' },
    });
    mockExportPrivateKey.mockResolvedValue('nsec1examplekey');

    renderDialog();

    await user.click(screen.getByRole('button', { name: '秘密鍵を取得' }));

    await waitFor(() => {
      expect(mockExportPrivateKey).toHaveBeenCalledWith('npub1example');
    });
    expect(screen.getByDisplayValue('nsec1examplekey')).toBeInTheDocument();
    expect(mockToast.success).toHaveBeenCalled();
    expect(mockErrorHandler.info).toHaveBeenCalled();
  });

  it('saves exported key to a file', async () => {
    const user = userEvent.setup();
    setupAuthStore({
      currentUser: { npub: 'npub1example' },
    });
    mockExportPrivateKey.mockResolvedValue('nsec1fileexport');
    mockDialogSave.mockResolvedValue('C:/temp/key.nsec');
    mockWriteTextFile.mockResolvedValue(undefined);

    renderDialog();
    await user.click(screen.getByRole('button', { name: '秘密鍵を取得' }));
    await waitFor(() => {
      expect(mockExportPrivateKey).toHaveBeenCalled();
    });

    await user.click(screen.getByRole('button', { name: 'ファイルに保存' }));

    await waitFor(() => {
      expect(mockDialogSave).toHaveBeenCalled();
      expect(mockWriteTextFile).toHaveBeenCalledWith('C:/temp/key.nsec', 'nsec1fileexport');
    });
    const history = useKeyManagementStore.getState().history;
    expect(history.some((entry) => entry.metadata?.stage === 'save-file')).toBe(true);
  });

  it('imports a key from manual input', async () => {
    const user = userEvent.setup();
    const loginWithNsec = vi.fn();
    setupAuthStore({
      currentUser: { npub: 'npub1example' },
      loginWithNsec,
    });

    renderDialog();
    await user.click(screen.getByRole('tab', { name: 'インポート' }));
    await user.type(screen.getByLabelText('秘密鍵を貼り付け'), 'nsec1manuallyimported');
    await user.click(screen.getByRole('button', { name: 'セキュアストレージに追加' }));

    await waitFor(() => {
      expect(loginWithNsec).toHaveBeenCalledWith('nsec1manuallyimported', true);
    });
    const history = useKeyManagementStore.getState().history;
    expect(history[0]?.action).toBe('import');
    expect(history[0]?.status).toBe('success');
  });

  it('loads a key from file selection', async () => {
    const user = userEvent.setup();
    setupAuthStore({
      currentUser: { npub: 'npub1example' },
    });
    mockDialogOpen.mockResolvedValue('C:/backup/key.nsec');
    mockReadTextFile.mockResolvedValue('nsec1fromfile');

    renderDialog();
    await user.click(screen.getByRole('tab', { name: 'インポート' }));
    await user.click(screen.getByRole('button', { name: '鍵ファイルを選択' }));

    await waitFor(() => {
      expect(mockDialogOpen).toHaveBeenCalled();
      expect(mockReadTextFile).toHaveBeenCalledWith('C:/backup/key.nsec');
      expect(screen.getByLabelText('秘密鍵を貼り付け')).toHaveValue('nsec1fromfile');
    });
    expect(mockToast.success).toHaveBeenCalledWith('秘密鍵ファイルを読み込みました');
  });
});
