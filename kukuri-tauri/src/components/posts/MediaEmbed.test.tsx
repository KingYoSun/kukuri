import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import MediaEmbed from './MediaEmbed';

describe('MediaEmbed', () => {
  it('renders YouTube embed for YouTube URL', () => {
    render(<MediaEmbed url="https://www.youtube.com/watch?v=dQw4w9WgXcQ" />);
    
    const iframe = screen.getByTitle('Embedded youtube content');
    expect(iframe).toBeInTheDocument();
    expect(iframe).toHaveAttribute('src', 'https://www.youtube.com/embed/dQw4w9WgXcQ');
  });

  it('renders YouTube embed for short YouTube URL', () => {
    render(<MediaEmbed url="https://youtu.be/dQw4w9WgXcQ" />);
    
    const iframe = screen.getByTitle('Embedded youtube content');
    expect(iframe).toBeInTheDocument();
    expect(iframe).toHaveAttribute('src', 'https://www.youtube.com/embed/dQw4w9WgXcQ');
  });

  it('renders Vimeo embed for Vimeo URL', () => {
    render(<MediaEmbed url="https://vimeo.com/123456789" />);
    
    const iframe = screen.getByTitle('Embedded vimeo content');
    expect(iframe).toBeInTheDocument();
    expect(iframe).toHaveAttribute('src', 'https://player.vimeo.com/video/123456789');
  });

  it('renders Twitter embed for Twitter URL', () => {
    render(<MediaEmbed url="https://twitter.com/user/status/1234567890" />);
    
    const iframe = screen.getByTitle('Embedded twitter content');
    expect(iframe).toBeInTheDocument();
    expect(iframe).toHaveAttribute('src', 'https://platform.twitter.com/embed/Tweet.html?id=1234567890');
  });

  it('renders Twitter embed for X.com URL', () => {
    render(<MediaEmbed url="https://x.com/user/status/1234567890" />);
    
    const iframe = screen.getByTitle('Embedded twitter content');
    expect(iframe).toBeInTheDocument();
    expect(iframe).toHaveAttribute('src', 'https://platform.twitter.com/embed/Tweet.html?id=1234567890');
  });

  it('renders image for image URL', () => {
    render(<MediaEmbed url="https://example.com/image.jpg" />);
    
    const img = screen.getByAltText('Embedded image');
    expect(img).toBeInTheDocument();
    expect(img).toHaveAttribute('src', 'https://example.com/image.jpg');
    expect(img).toHaveClass('max-w-full', 'h-auto', 'rounded-lg');
  });

  it('renders image for various image formats', () => {
    const formats = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg'];
    
    formats.forEach(format => {
      const { unmount } = render(<MediaEmbed url={`https://example.com/image.${format}`} />);
      
      const img = screen.getByAltText('Embedded image');
      expect(img).toBeInTheDocument();
      expect(img).toHaveAttribute('src', `https://example.com/image.${format}`);
      
      unmount();
    });
  });

  it('renders link for unrecognized URL', () => {
    render(<MediaEmbed url="https://example.com/page" />);
    
    const link = screen.getByText('https://example.com/page');
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute('href', 'https://example.com/page');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('applies custom className', () => {
    render(<MediaEmbed url="https://example.com/image.jpg" className="custom-class" />);
    
    const img = screen.getByAltText('Embedded image');
    expect(img).toHaveClass('custom-class');
  });

  it('sets correct aspect ratio for video embeds', () => {
    const { container } = render(<MediaEmbed url="https://www.youtube.com/watch?v=test" />);
    
    const wrapper = container.firstChild;
    expect(wrapper).toHaveStyle({ aspectRatio: '16/9' });
  });
});