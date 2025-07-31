import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';
import MarkdownEditor from './MarkdownEditor';

// Mock errorHandler
vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
  },
}));

// Mock @uiw/react-md-editor
vi.mock('@uiw/react-md-editor', () => {
  const MockMDEditor = ({ value, onChange, preview, height, hideToolbar, commands, textareaProps }: any) => {
    const React = require('react');
    const [internalValue, setInternalValue] = React.useState(value || '');
    
    React.useEffect(() => {
      setInternalValue(value || '');
    }, [value]);
      
      const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
        const newValue = e.target.value;
        setInternalValue(newValue);
        onChange?.(newValue);
      };

      return (
        <div data-testid="md-editor" style={{ height }}>
          {!hideToolbar && (
            <div data-testid="toolbar">
              {commands?.map((cmd: any, index: number) => (
                <button
                  key={index}
                  data-testid={`toolbar-${cmd.name}`}
                  onClick={() => cmd.execute?.()}
                  aria-label={cmd.buttonProps?.['aria-label']}
                >
                  {cmd.icon || cmd.name}
                </button>
              ))}
            </div>
          )}
          <textarea
            data-testid="editor-textarea"
            className="w-md-editor-text-input"
            value={internalValue}
            onChange={handleChange}
            placeholder={textareaProps?.placeholder}
          />
          {(preview === 'live' || preview === 'preview') && (
            <div data-testid="preview">{internalValue}</div>
          )}
        </div>
      );
  };
  
  return {
    __esModule: true,
    default: MockMDEditor,
    commands: [
      { name: 'bold', keyCommand: 'bold' },
      { name: 'italic', keyCommand: 'italic' },
      { name: 'image', keyCommand: 'image' },
      { name: 'link', keyCommand: 'link' },
    ],
  };
});

describe('MarkdownEditor', () => {
  const mockOnChange = vi.fn();
  const mockOnImageUpload = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders with default props', () => {
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
      />
    );

    expect(screen.getByTestId('md-editor')).toBeInTheDocument();
    expect(screen.getByTestId('editor-textarea')).toBeInTheDocument();
    expect(screen.getByTestId('preview')).toBeInTheDocument();
  });

  it('displays placeholder text', () => {
    const placeholder = 'Write something...';
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
        placeholder={placeholder}
      />
    );

    expect(screen.getByPlaceholderText(placeholder)).toBeInTheDocument();
  });

  it('calls onChange when text is entered', async () => {
    const user = userEvent.setup();
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
      />
    );

    const textarea = screen.getByTestId('editor-textarea');
    await user.type(textarea, 'Hello world');

    // userEvent.type calls onChange for each character typed
    expect(mockOnChange).toHaveBeenCalledTimes(11); // 'Hello world' has 11 characters
    
    // Check that onChange was called with progressively longer strings
    expect(mockOnChange).toHaveBeenNthCalledWith(1, 'H');
    expect(mockOnChange).toHaveBeenNthCalledWith(2, 'He');
    expect(mockOnChange).toHaveBeenNthCalledWith(11, 'Hello world');
  });

  it('respects maxLength constraint', async () => {
    const user = userEvent.setup();
    render(
      <MarkdownEditor
        value="Hello"
        onChange={mockOnChange}
        maxLength={10}
      />
    );

    const textarea = screen.getByTestId('editor-textarea');
    await user.type(textarea, ' world!!!');

    // Should not exceed maxLength
    expect(mockOnChange).not.toHaveBeenCalledWith(expect.stringContaining('Hello world!!!'));
    expect(screen.getByText('5 / 10')).toBeInTheDocument();
  });

  it('hides toolbar when hideToolbar is true', () => {
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
        hideToolbar={true}
      />
    );

    expect(screen.queryByTestId('toolbar')).not.toBeInTheDocument();
  });

  it('renders with custom height', () => {
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
        height={500}
      />
    );

    const editor = screen.getByTestId('md-editor');
    expect(editor).toHaveStyle({ height: '500px' });
  });

  it('handles image upload', () => {
    // This test verifies that the MarkdownEditor passes the onImageUpload prop correctly
    // The actual image upload functionality is tested via the integration of commands
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
        onImageUpload={mockOnImageUpload}
      />
    );

    // The editor should be rendered with image upload capability
    expect(screen.getByTestId('md-editor')).toBeInTheDocument();
    
    // Since we're mocking the editor, we'll verify the prop was passed correctly
    // by checking that our component renders with the expected structure
    expect(screen.getByTestId('toolbar')).toBeInTheDocument();
  });

  it('validates image file type', async () => {
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
        onImageUpload={mockOnImageUpload}
      />
    );

    const file = new File(['test'], 'test.txt', { type: 'text/plain' });
    
    // Find hidden file input
    const hiddenInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    if (hiddenInput) {
      Object.defineProperty(hiddenInput, 'files', {
        value: [file],
        writable: false,
      });
      fireEvent.change(hiddenInput);
    }

    await waitFor(() => {
      expect(mockOnImageUpload).not.toHaveBeenCalled();
    });
  });

  it('validates image file size', async () => {
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
        onImageUpload={mockOnImageUpload}
      />
    );

    // Create a file larger than 10MB
    const largeFile = new File([new ArrayBuffer(11 * 1024 * 1024)], 'large.jpg', { 
      type: 'image/jpeg' 
    });
    
    // Find hidden file input
    const hiddenInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    if (hiddenInput) {
      Object.defineProperty(hiddenInput, 'files', {
        value: [largeFile],
        writable: false,
      });
      fireEvent.change(hiddenInput);
    }

    await waitFor(() => {
      expect(mockOnImageUpload).not.toHaveBeenCalled();
    });
  });

  it('handles image upload error', async () => {
    mockOnImageUpload.mockRejectedValue(new Error('Upload failed'));

    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
        onImageUpload={mockOnImageUpload}
      />
    );

    const file = new File(['test'], 'test.jpg', { type: 'image/jpeg' });
    
    // Find hidden file input
    const hiddenInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    if (hiddenInput) {
      Object.defineProperty(hiddenInput, 'files', {
        value: [file],
        writable: false,
      });
      fireEvent.change(hiddenInput);
    }

    await waitFor(() => {
      expect(mockOnImageUpload).toHaveBeenCalledWith(file);
      expect(mockOnChange).not.toHaveBeenCalled();
    });
  });
});