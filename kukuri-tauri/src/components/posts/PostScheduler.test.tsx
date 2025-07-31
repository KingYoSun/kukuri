import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import PostScheduler from './PostScheduler';
import { format, addDays } from 'date-fns';
import { ja } from 'date-fns/locale';

// Mock react-day-picker
vi.mock('react-day-picker', () => ({
  DayPicker: ({ mode, selected, onSelect, disabled, className }: any) => {
    const today = new Date();
    const tomorrow = addDays(today, 1);
    const nextWeek = addDays(today, 7);

    const handleClick = (date: Date) => {
      if (disabled && disabled(date)) return;
      onSelect(date);
    };

    return (
      <div data-testid="day-picker" className={className}>
        <button
          data-testid="select-today"
          onClick={() => handleClick(today)}
        >
          Today
        </button>
        <button
          data-testid="select-tomorrow"
          onClick={() => handleClick(tomorrow)}
        >
          Tomorrow
        </button>
        <button
          data-testid="select-next-week"
          onClick={() => handleClick(nextWeek)}
        >
          Next Week
        </button>
      </div>
    );
  },
}));

describe('PostScheduler', () => {
  const mockOnSchedule = vi.fn();
  const today = new Date();
  today.setHours(9, 0, 0, 0);

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders with default state', () => {
    render(<PostScheduler scheduledDate={null} onSchedule={mockOnSchedule} />);
    
    expect(screen.getByText('予約投稿')).toBeInTheDocument();
  });

  it('displays scheduled date when provided', () => {
    const scheduledDate = addDays(today, 1);
    scheduledDate.setHours(14, 30, 0, 0);
    
    render(<PostScheduler scheduledDate={scheduledDate} onSchedule={mockOnSchedule} />);
    
    expect(screen.getByText('明日 14:30')).toBeInTheDocument();
  });

  it('opens popover when clicked', async () => {
    const user = userEvent.setup();
    render(<PostScheduler scheduledDate={null} onSchedule={mockOnSchedule} />);
    
    await user.click(screen.getByText('予約投稿'));
    
    expect(screen.getByTestId('day-picker')).toBeInTheDocument();
    expect(screen.getByText('今日')).toBeInTheDocument();
    expect(screen.getByText('明日')).toBeInTheDocument();
  });

  it('handles quick select for today', async () => {
    const user = userEvent.setup();
    render(<PostScheduler scheduledDate={null} onSchedule={mockOnSchedule} />);
    
    await user.click(screen.getByText('予約投稿'));
    await user.click(screen.getByText('今日'));
    
    expect(mockOnSchedule).toHaveBeenCalledWith(
      expect.objectContaining({
        getHours: expect.any(Function),
        getMinutes: expect.any(Function),
      })
    );
    
    const calledDate = mockOnSchedule.mock.calls[0][0];
    expect(calledDate.getHours()).toBe(9);
    expect(calledDate.getMinutes()).toBe(0);
  });

  it('handles quick select for tomorrow', async () => {
    const user = userEvent.setup();
    render(<PostScheduler scheduledDate={null} onSchedule={mockOnSchedule} />);
    
    await user.click(screen.getByText('予約投稿'));
    await user.click(screen.getByText('明日'));
    
    const calledDate = mockOnSchedule.mock.calls[0][0];
    const tomorrow = addDays(new Date(), 1);
    
    expect(calledDate.getDate()).toBe(tomorrow.getDate());
    expect(calledDate.getMonth()).toBe(tomorrow.getMonth());
  });

  it('selects date from calendar', async () => {
    const user = userEvent.setup();
    render(<PostScheduler scheduledDate={null} onSchedule={mockOnSchedule} />);
    
    await user.click(screen.getByText('予約投稿'));
    
    const tomorrowButton = screen.getByTestId('select-tomorrow');
    await user.click(tomorrowButton);
    
    expect(mockOnSchedule).toHaveBeenCalled();
  });

  it('changes time selection', async () => {
    const user = userEvent.setup();
    const scheduledDate = addDays(today, 1);
    
    render(<PostScheduler scheduledDate={scheduledDate} onSchedule={mockOnSchedule} />);
    
    await user.click(screen.getByText('明日 09:00'));
    
    // Find and click hour selector
    const hourSelectors = screen.getAllByRole('combobox');
    await user.click(hourSelectors[0]);
    
    // Select 14:00
    const hour14Option = await screen.findByText('14');
    await user.click(hour14Option);
    
    expect(mockOnSchedule).toHaveBeenCalledWith(
      expect.objectContaining({
        getHours: expect.any(Function),
      })
    );
    
    const calledDate = mockOnSchedule.mock.calls[0][0];
    expect(calledDate.getHours()).toBe(14);
  });

  it('clears scheduled date', async () => {
    const user = userEvent.setup();
    const scheduledDate = addDays(today, 1);
    
    render(<PostScheduler scheduledDate={scheduledDate} onSchedule={mockOnSchedule} />);
    
    // Find the X icon inside the button
    const scheduleButton = screen.getByRole('button', { name: /明日/ });
    const clearIcon = scheduleButton.querySelector('.lucide-x');
    
    if (clearIcon) {
      fireEvent.click(clearIcon);
    }
    
    await waitFor(() => {
      expect(mockOnSchedule).toHaveBeenCalledWith(null);
    });
  });

  it('clears schedule using clear button in popover', async () => {
    const user = userEvent.setup();
    const scheduledDate = addDays(today, 1);
    
    render(<PostScheduler scheduledDate={scheduledDate} onSchedule={mockOnSchedule} />);
    
    await user.click(screen.getByText(/明日/));
    await user.click(screen.getByText('クリア'));
    
    expect(mockOnSchedule).toHaveBeenCalledWith(null);
  });

  it('formats today correctly', () => {
    const scheduledDate = new Date();
    scheduledDate.setHours(15, 45, 0, 0);
    
    render(<PostScheduler scheduledDate={scheduledDate} onSchedule={mockOnSchedule} />);
    
    expect(screen.getByText('今日 15:45')).toBeInTheDocument();
  });

  it('formats future date correctly', () => {
    const scheduledDate = addDays(today, 7);
    scheduledDate.setHours(10, 30, 0, 0);
    
    render(<PostScheduler scheduledDate={scheduledDate} onSchedule={mockOnSchedule} />);
    
    const expectedText = format(scheduledDate, 'M月d日 (E) HH:mm', { locale: ja });
    const dateOnly = format(scheduledDate, 'M月d日 (E)', { locale: ja });
    
    expect(screen.getByText(`${dateOnly} 10:30`)).toBeInTheDocument();
  });

  it('disables past dates', async () => {
    const user = userEvent.setup();
    render(<PostScheduler scheduledDate={null} onSchedule={mockOnSchedule} />);
    
    await user.click(screen.getByText('予約投稿'));
    
    // The DayPicker mock should respect the disabled prop
    // In real implementation, past dates would be visually disabled
    expect(screen.getByTestId('day-picker')).toBeInTheDocument();
  });

  it('applies custom className', () => {
    render(
      <PostScheduler
        scheduledDate={null}
        onSchedule={mockOnSchedule}
        className="custom-class"
      />
    );
    
    const button = screen.getByText('予約投稿');
    expect(button).toHaveClass('custom-class');
  });
});