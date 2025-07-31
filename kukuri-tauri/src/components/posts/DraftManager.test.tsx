import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import DraftManager from './DraftManager';
import { useDraftStore } from '@/stores/draftStore';
import type { PostDraft } from '@/types/draft';

// Mock the draft store
vi.mock('@/stores/draftStore');

const mockDrafts: PostDraft[] = [
  {
    id: 'draft1',
    content: 'This is the first draft content',
    topicId: 'topic1',
    topicName: 'Technology',
    scheduledDate: new Date('2025-08-01T10:00:00'),
    createdAt: new Date('2025-07-30T08:00:00'),
    updatedAt: new Date('2025-07-30T09:00:00'),
  },
  {
    id: 'draft2',
    content: 'Second draft without topic',
    topicId: null,
    scheduledDate: null,
    createdAt: new Date('2025-07-29T10:00:00'),
    updatedAt: new Date('2025-07-29T11:00:00'),
  },
  {
    id: 'draft3',
    content: 'A very long draft content that should be truncated in the preview. '.repeat(5),
    topicId: 'topic2',
    topicName: 'Life',
    scheduledDate: null,
    createdAt: new Date('2025-07-28T12:00:00'),
    updatedAt: new Date('2025-07-31T13:00:00'),
  },
];

describe('DraftManager', () => {
  const mockOnSelectDraft = vi.fn();
  const mockDeleteDraft = vi.fn();
  const mockClearAllDrafts = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useDraftStore).mockReturnValue({
      drafts: mockDrafts,
      deleteDraft: mockDeleteDraft,
      clearAllDrafts: mockClearAllDrafts,
      currentDraftId: null,
      createDraft: vi.fn(),
      updateDraft: vi.fn(),
      getDraft: vi.fn(),
      setCurrentDraft: vi.fn(),
      getCurrentDraft: vi.fn(),
      listDrafts: vi.fn(),
      autosaveDraft: vi.fn(),
    });
  });

  it('renders empty state when no drafts', () => {
    vi.mocked(useDraftStore).mockReturnValue({
      drafts: [],
      deleteDraft: mockDeleteDraft,
      clearAllDrafts: mockClearAllDrafts,
      currentDraftId: null,
      createDraft: vi.fn(),
      updateDraft: vi.fn(),
      getDraft: vi.fn(),
      setCurrentDraft: vi.fn(),
      getCurrentDraft: vi.fn(),
      listDrafts: vi.fn(),
      autosaveDraft: vi.fn(),
    });

    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    expect(screen.getByText('下書きはありません')).toBeInTheDocument();
  });

  it('renders list of drafts', () => {
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    expect(screen.getByText('下書き一覧')).toBeInTheDocument();
    expect(screen.getByText(/This is the first draft content/)).toBeInTheDocument();
    expect(screen.getByText(/Second draft without topic/)).toBeInTheDocument();
  });

  it('displays topic name when available', () => {
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    expect(screen.getByText('Technology')).toBeInTheDocument();
    expect(screen.getByText('Life')).toBeInTheDocument();
  });

  it('displays scheduled date when available', () => {
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    expect(screen.getByText(/予約: 8月1日 10:00/)).toBeInTheDocument();
  });

  it('truncates long content in preview', () => {
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    const longDraftPreview = screen.getByText(/A very long draft content/);
    expect(longDraftPreview.textContent).toContain('...');
    expect(longDraftPreview.textContent!.length).toBeLessThan(150);
  });

  it('calls onSelectDraft when draft is clicked', async () => {
    const user = userEvent.setup();
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    const firstDraft = screen.getByText(/This is the first draft content/).closest('.cursor-pointer');
    await user.click(firstDraft!);
    
    expect(mockOnSelectDraft).toHaveBeenCalledWith(mockDrafts[0]);
  });

  it('calls onSelectDraft when edit button is clicked', async () => {
    const user = userEvent.setup();
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    const editButtons = screen.getAllByRole('button', { name: '' });
    const firstEditButton = editButtons.find(btn => btn.querySelector('svg'));
    
    await user.click(firstEditButton!);
    
    expect(mockOnSelectDraft).toHaveBeenCalledWith(mockDrafts[0]);
  });

  it('shows delete confirmation dialog when delete button is clicked', async () => {
    const user = userEvent.setup();
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    const deleteButtons = screen.getAllByRole('button').filter(btn => 
      btn.className.includes('text-destructive') && btn.querySelector('svg')
    );
    
    await user.click(deleteButtons[0]);
    
    expect(screen.getByText('下書きを削除')).toBeInTheDocument();
    expect(screen.getByText('この下書きを削除してもよろしいですか？この操作は取り消せません。')).toBeInTheDocument();
  });

  it('deletes draft when confirmed', async () => {
    const user = userEvent.setup();
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    const deleteButtons = screen.getAllByRole('button').filter(btn => 
      btn.className.includes('text-destructive') && btn.querySelector('svg')
    );
    
    await user.click(deleteButtons[0]);
    
    const confirmButton = screen.getByRole('button', { name: '削除' });
    await user.click(confirmButton);
    
    expect(mockDeleteDraft).toHaveBeenCalledWith('draft1');
  });

  it('cancels delete when cancel is clicked', async () => {
    const user = userEvent.setup();
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    const deleteButtons = screen.getAllByRole('button').filter(btn => 
      btn.className.includes('text-destructive') && btn.querySelector('svg')
    );
    
    await user.click(deleteButtons[0]);
    
    const cancelButton = screen.getByRole('button', { name: 'キャンセル' });
    await user.click(cancelButton);
    
    expect(mockDeleteDraft).not.toHaveBeenCalled();
  });

  it('shows clear all confirmation dialog', async () => {
    const user = userEvent.setup();
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    const clearAllButton = screen.getByRole('button', { name: 'すべて削除' });
    await user.click(clearAllButton);
    
    expect(screen.getByText('すべての下書きを削除')).toBeInTheDocument();
    expect(screen.getByText('すべての下書きを削除してもよろしいですか？この操作は取り消せません。')).toBeInTheDocument();
  });

  it('clears all drafts when confirmed', async () => {
    const user = userEvent.setup();
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    const clearAllButton = screen.getByRole('button', { name: 'すべて削除' });
    await user.click(clearAllButton);
    
    // Wait for dialog to appear
    await waitFor(() => {
      expect(screen.getByText('すべての下書きを削除してもよろしいですか？この操作は取り消せません。')).toBeInTheDocument();
    });
    
    // Find the AlertDialogAction button within the dialog by its specific class
    const confirmButton = screen.getByRole('button', {
      name: 'すべて削除',
      // Use a more specific selector to get the dialog button
    }).parentElement?.querySelector('.bg-destructive') as HTMLButtonElement;
    
    if (confirmButton) {
      fireEvent.click(confirmButton);
    } else {
      // Fallback: try to find the second button with this text
      const allButtons = screen.getAllByRole('button', { name: 'すべて削除' });
      fireEvent.click(allButtons[1]);
    }
    
    await waitFor(() => {
      expect(mockClearAllDrafts).toHaveBeenCalled();
    });
  });

  it('applies custom className', () => {
    const { container } = render(
      <DraftManager onSelectDraft={mockOnSelectDraft} className="custom-class" />
    );
    
    const card = container.querySelector('.custom-class');
    expect(card).toBeInTheDocument();
  });

  it('displays updated time correctly', () => {
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    expect(screen.getByText(/更新: 7月30日 09:00/)).toBeInTheDocument();
    expect(screen.getByText(/更新: 7月29日 11:00/)).toBeInTheDocument();
    expect(screen.getByText(/更新: 7月31日 13:00/)).toBeInTheDocument();
  });

  it('prevents event propagation when clicking action buttons', async () => {
    const user = userEvent.setup();
    render(<DraftManager onSelectDraft={mockOnSelectDraft} />);
    
    const deleteButtons = screen.getAllByRole('button').filter(btn => 
      btn.className.includes('text-destructive') && btn.querySelector('svg')
    );
    
    await user.click(deleteButtons[0]);
    
    // onSelectDraft should not be called when clicking delete button
    expect(mockOnSelectDraft).not.toHaveBeenCalled();
  });
});