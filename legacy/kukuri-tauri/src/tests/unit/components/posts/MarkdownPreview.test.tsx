import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import MarkdownPreview from '@/components/posts/MarkdownPreview';

// Mock MediaEmbed component
vi.mock('@/components/posts/MediaEmbed', () => ({
  __esModule: true,
  default: ({ url, className }: { url: string; className?: string }) => (
    <div data-testid="media-embed" data-url={url} data-embed="media-embed" className={className}>
      MediaEmbed: {url}
    </div>
  ),
}));

describe('MarkdownPreview', () => {
  it('renders basic markdown text', () => {
    render(<MarkdownPreview content="Hello **world**!" />);

    const text = screen.getByText(/Hello/);
    const bold = screen.getByText('world');

    expect(text).toBeInTheDocument();
    expect(bold.tagName).toBe('STRONG');
  });

  it('renders headings', () => {
    const content = `
# Heading 1
## Heading 2
### Heading 3
    `;

    render(<MarkdownPreview content={content} />);

    expect(screen.getByText('Heading 1').tagName).toBe('H1');
    expect(screen.getByText('Heading 2').tagName).toBe('H2');
    expect(screen.getByText('Heading 3').tagName).toBe('H3');
  });

  it('renders lists', () => {
    const content = `
- Item 1
- Item 2
- Item 3

1. First
2. Second
3. Third
    `;

    render(<MarkdownPreview content={content} />);

    expect(screen.getByText('Item 1')).toBeInTheDocument();
    expect(screen.getByText('Item 2')).toBeInTheDocument();
    expect(screen.getByText('First')).toBeInTheDocument();
    expect(screen.getByText('Second')).toBeInTheDocument();
  });

  it('renders links with target="_blank"', () => {
    render(<MarkdownPreview content="[Google](https://google.com)" />);

    const link = screen.getByText('Google');
    expect(link).toHaveAttribute('href', 'https://google.com');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('embeds YouTube URLs', () => {
    render(<MarkdownPreview content="https://www.youtube.com/watch?v=test" />);

    const embed = screen.getByTestId('media-embed');
    expect(embed).toBeInTheDocument();
    expect(embed).toHaveAttribute('data-url', 'https://www.youtube.com/watch?v=test');
  });

  it('does not embed media URLs when they have custom text', () => {
    render(<MarkdownPreview content="[Watch this video](https://www.youtube.com/watch?v=test)" />);

    expect(screen.queryByTestId('media-embed')).not.toBeInTheDocument();

    const link = screen.getByText('Watch this video');
    expect(link).toHaveAttribute('href', 'https://www.youtube.com/watch?v=test');
  });

  it('renders images', () => {
    render(<MarkdownPreview content="![Alt text](https://example.com/image.jpg)" />);

    const img = screen.getByAltText('Alt text');
    expect(img).toBeInTheDocument();
    expect(img).toHaveAttribute('src', 'https://example.com/image.jpg');
    expect(img).toHaveClass('max-w-full', 'h-auto', 'rounded-lg');
  });

  it('renders code blocks with syntax highlighting class', () => {
    const content = '```javascript\nconst hello = "world";\n```';

    render(<MarkdownPreview content={content} />);

    const codeBlock = screen.getByText('const hello = "world";');
    expect(codeBlock.tagName).toBe('CODE');
    expect(codeBlock.parentElement?.tagName).toBe('PRE');
    expect(codeBlock.parentElement).toHaveClass('bg-muted', 'rounded-lg');
  });

  it('renders inline code', () => {
    render(<MarkdownPreview content="Use `npm install` to install" />);

    const code = screen.getByText('npm install');
    expect(code.tagName).toBe('CODE');
    expect(code).toHaveClass('bg-muted', 'px-1', 'py-0.5', 'rounded');
  });

  it('renders blockquotes with custom styling', () => {
    render(<MarkdownPreview content="> This is a quote" />);

    const blockquote = screen.getByText('This is a quote').parentElement;
    expect(blockquote?.tagName).toBe('BLOCKQUOTE');
    expect(blockquote).toHaveClass('border-l-4', 'border-primary', 'pl-4', 'italic');
  });

  it('renders tables with custom styling', () => {
    const content = `
| Header 1 | Header 2 |
|----------|----------|
| Cell 1   | Cell 2   |
    `;

    render(<MarkdownPreview content={content} />);

    const table = screen.getByRole('table');
    expect(table).toHaveClass('min-w-full', 'divide-y', 'divide-border');

    const th = screen.getByText('Header 1');
    expect(th.tagName).toBe('TH');
    expect(th).toHaveClass('px-4', 'py-2', 'text-left');

    const td = screen.getByText('Cell 1');
    expect(td.tagName).toBe('TD');
    expect(td).toHaveClass('px-4', 'py-2', 'text-sm');
  });

  it('applies custom className', () => {
    const { container } = render(<MarkdownPreview content="Hello" className="custom-class" />);

    const wrapper = container.firstChild;
    expect(wrapper).toHaveClass('custom-class');
  });

  it('renders GitHub Flavored Markdown features', () => {
    const content = `
~~Strikethrough~~

- [x] Completed task
- [ ] Incomplete task
    `;

    render(<MarkdownPreview content={content} />);

    // Strikethrough
    const strikethrough = screen.getByText('Strikethrough');
    expect(strikethrough.tagName).toBe('DEL');

    // Task lists
    expect(screen.getByText('Completed task')).toBeInTheDocument();
    expect(screen.getByText('Incomplete task')).toBeInTheDocument();
  });
});
