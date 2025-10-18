import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi, describe, it, expect, beforeEach, MockedFunction } from 'vitest';
import { NostrTestPanel } from '../NostrTestPanel';
import type { CommandResponse } from '@/lib/api/tauriClient';

// Tauri APIをモック
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

// Zustand storeをモック
vi.mock('@/stores/authStore', () => ({
  useAuthStore: vi.fn(),
}));

// sonnerのtoastをモック
vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAuthStore } from '@/stores/authStore';
import { toast } from 'sonner';

const mockInvoke = invoke as MockedFunction<typeof invoke>;
const mockListen = listen as MockedFunction<typeof listen>;
const mockUseAuthStore = useAuthStore as unknown as MockedFunction<typeof useAuthStore>;

const successResponse = <T,>(data: T): CommandResponse<T> => ({
  success: true,
  data,
  error: null,
  error_code: null,
});

describe('NostrTestPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // デフォルトでログイン状態をモック
    mockUseAuthStore.mockReturnValue({
      isAuthenticated: true,
    });

    // listenモックのデフォルト実装
    const mockUnlisten = vi.fn();
    mockListen.mockResolvedValue(mockUnlisten);
  });

  it('renders test panel with all sections', () => {
    render(<NostrTestPanel />);

    expect(screen.getByText('Nostrイベント送受信テスト')).toBeInTheDocument();
    expect(screen.getByText('送信テスト')).toBeInTheDocument();
    expect(screen.getByText('購読テスト')).toBeInTheDocument();
    expect(screen.getByText('実行ログ')).toBeInTheDocument();
    expect(screen.getByText('受信イベント')).toBeInTheDocument();
  });

  it('sends text note when button is clicked', async () => {
    const mockEventId = 'test-event-id-123';
    mockInvoke.mockResolvedValueOnce(
      successResponse({ event_id: mockEventId, success: true, message: null }),
    );

    render(<NostrTestPanel />);

    const testInput = screen.getByPlaceholderText('テストメッセージを入力');
    await userEvent.type(testInput, 'Test note from Nostr Test Panel');

    const sendNoteButton = screen.getByRole('button', { name: 'テキストノート送信' });
    await userEvent.click(sendNoteButton);

    expect(invoke).toHaveBeenCalledWith('publish_text_note', {
      content: 'Test note from Nostr Test Panel',
    });

    // 実行ログタブに切り替え
    const logTab = screen.getByRole('tab', { name: '実行ログ' });
    await userEvent.click(logTab);

    await waitFor(() => {
      expect(screen.getByText(new RegExp(mockEventId))).toBeInTheDocument();
    });
  });

  it('sends topic post with form data', async () => {
    const mockEventId = 'topic-event-id-456';
    mockInvoke.mockResolvedValueOnce(
      successResponse({ event_id: mockEventId, success: true, message: null }),
    );

    render(<NostrTestPanel />);

    const topicInput = screen.getByPlaceholderText('トピックID');
    const contentInput = screen.getByPlaceholderText('テストメッセージを入力');

    await userEvent.clear(topicInput);
    await userEvent.type(topicInput, 'nostr');
    await userEvent.type(contentInput, 'Nostr is awesome!');

    const sendButton = screen.getByRole('button', { name: 'トピック投稿送信' });
    await userEvent.click(sendButton);

    expect(invoke).toHaveBeenCalledWith('publish_topic_post', {
      topicId: 'nostr',
      content: 'Nostr is awesome!',
      replyTo: null,
    });

    // 実行ログタブに切り替え
    const logTab = screen.getByRole('tab', { name: '実行ログ' });
    await userEvent.click(logTab);

    await waitFor(() => {
      expect(screen.getByText(new RegExp(mockEventId))).toBeInTheDocument();
    });
  });

  it('sends reaction with form data', async () => {
    const mockReactionId = 'reaction-id-789';
    mockInvoke.mockResolvedValueOnce(
      successResponse({ event_id: mockReactionId, success: true, message: null }),
    );

    render(<NostrTestPanel />);

    // リアクション送信ボタンをクリックしてpromptをモック
    window.prompt = vi.fn().mockReturnValue('target-event-id');

    const sendButton = screen.getByRole('button', { name: 'リアクション送信' });
    await userEvent.click(sendButton);

    expect(invoke).toHaveBeenCalledWith('send_reaction', {
      eventId: 'target-event-id',
      reaction: '+',
    });

    // 実行ログタブに切り替え
    const logTab = screen.getByRole('tab', { name: '実行ログ' });
    await userEvent.click(logTab);

    await waitFor(() => {
      expect(screen.getByText(new RegExp(mockReactionId))).toBeInTheDocument();
    });
  });

  it('subscribes to topic', async () => {
    mockInvoke.mockResolvedValueOnce(successResponse(null));

    render(<NostrTestPanel />);

    // 購読テストタブに切り替え
    const subscribeTab = screen.getByRole('tab', { name: '購読テスト' });
    await userEvent.click(subscribeTab);

    const topicInput = screen.getByPlaceholderText('トピックID');
    await userEvent.clear(topicInput);
    await userEvent.type(topicInput, 'technology');

    const subscribeButton = screen.getByRole('button', { name: 'トピックを購読' });
    await userEvent.click(subscribeButton);

    expect(invoke).toHaveBeenCalledWith('subscribe_to_topic', {
      topicId: 'technology',
    });

    // 実行ログタブに切り替え
    const logTab = screen.getByRole('tab', { name: '実行ログ' });
    await userEvent.click(logTab);

    await waitFor(() => {
      expect(screen.getByText(/✅ トピック購読成功: technology/)).toBeInTheDocument();
    });
  });

  it('displays received events', async () => {
    const mockUnlisten = vi.fn();
    mockListen.mockResolvedValueOnce(mockUnlisten);

    render(<NostrTestPanel />);

    // イベントリスナーが設定されていることを確認
    expect(listen).toHaveBeenCalledWith('nostr://event', expect.any(Function));

    // リスナーコールバックを取得してテスト
    const listenerCallback = mockListen.mock.calls[0][1] as (event: unknown) => void;

    const mockEvent = {
      payload: {
        id: 'received-event-id',
        author: 'test-author',
        content: 'Test received content',
        created_at: 1234567890,
        kind: 1,
        tags: [],
      },
    };

    // actでラップしてイベントを受信
    const { act } = await import('@testing-library/react');
    await act(async () => {
      listenerCallback(mockEvent);
    });

    // 受信イベントタブに切り替え
    const receivedTab = screen.getByRole('tab', { name: '受信イベント' });
    await userEvent.click(receivedTab);

    await waitFor(() => {
      expect(screen.getByText(/Test received content/)).toBeInTheDocument();
    });
  });

  it('handles errors gracefully', async () => {
    render(<NostrTestPanel />);

    // テキストを入力せずに送信ボタンがdisabledであることを確認
    const sendNoteButton = screen.getByRole('button', { name: 'テキストノート送信' });
    expect(sendNoteButton).toBeDisabled();

    // テキストを入力してからクリア
    const testInput = screen.getByPlaceholderText('テストメッセージを入力');
    await userEvent.type(testInput, 'Test');
    expect(sendNoteButton).not.toBeDisabled();

    // APIエラーをモック
    mockInvoke.mockRejectedValueOnce(new Error('Network error'));

    // 送信ボタンをクリック
    await userEvent.click(sendNoteButton);

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('送信に失敗しました');
    });
  });

  it('cleans up event listener on unmount', async () => {
    const mockUnlisten = vi.fn();
    mockListen.mockResolvedValueOnce(mockUnlisten);

    const { unmount } = render(<NostrTestPanel />);

    unmount();

    await waitFor(() => {
      expect(mockUnlisten).toHaveBeenCalled();
    });
  });
});
