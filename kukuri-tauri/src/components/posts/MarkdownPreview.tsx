import React from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeRaw from 'rehype-raw';
import { cn } from '@/lib/utils';
import MediaEmbed from './MediaEmbed';

interface MarkdownPreviewProps {
  content: string;
  className?: string;
}

const MarkdownPreview: React.FC<MarkdownPreviewProps> = ({ content, className }) => {
  return (
    <ReactMarkdown
      className={cn('prose prose-sm dark:prose-invert max-w-none', className)}
      remarkPlugins={[remarkGfm]}
      rehypePlugins={[rehypeRaw]}
      components={{
        // Custom link renderer to detect media URLs
        a: ({ node: _node, href, children, ...props }) => {
          if (!href) return <a {...props}>{children}</a>;

          // Check if this is a media URL that should be embedded
          const mediaPatterns = [
            /youtube\.com\/watch\?v=/,
            /youtu\.be\//,
            /vimeo\.com\/\d+/,
            /twitter\.com\/\w+\/status\/\d+/,
            /x\.com\/\w+\/status\/\d+/,
          ];

          const isMediaUrl = mediaPatterns.some(pattern => pattern.test(href));
          
          // If the link text is the same as href, it's likely meant to be embedded
          const shouldEmbed = isMediaUrl && (
            children?.toString() === href ||
            children?.toString().startsWith('http')
          );

          if (shouldEmbed) {
            return <MediaEmbed url={href} className="my-4" />;
          }

          return (
            <a href={href} target="_blank" rel="noopener noreferrer" {...props}>
              {children}
            </a>
          );
        },
        // Override paragraph to handle media embeds properly
        p: ({ node: _node, children, ...props }) => {
          // Check if the paragraph contains only a media embed
          const childrenArray = React.Children.toArray(children);
          if (childrenArray.length === 1) {
            const child = childrenArray[0];
            if (React.isValidElement(child) && child.type === MediaEmbed) {
              // Return the media embed without wrapping in a paragraph
              return child;
            }
          }
          
          return <p {...props}>{children}</p>;
        },
        // Custom image renderer
        img: ({ node: _node, src, alt, ...props }) => {
          if (!src) return null;
          
          return (
            <img
              src={src}
              alt={alt || 'Image'}
              className="max-w-full h-auto rounded-lg my-4"
              loading="lazy"
              {...props}
            />
          );
        },
        // Custom code block renderer
        code: ({ node: _node, inline, className, children, ...props }: any) => {
          const match = /language-(\w+)/.exec(className || '');
          
          if (!inline && match) {
            return (
              <pre className="bg-muted rounded-lg p-4 overflow-x-auto">
                <code className={className} {...props}>
                  {children}
                </code>
              </pre>
            );
          }
          
          return (
            <code className="bg-muted px-1 py-0.5 rounded text-sm" {...props}>
              {children}
            </code>
          );
        },
        // Custom blockquote renderer
        blockquote: ({ node: _node, children, ...props }) => {
          return (
            <blockquote
              className="border-l-4 border-primary pl-4 my-4 italic"
              {...props}
            >
              {children}
            </blockquote>
          );
        },
        // Custom table renderer
        table: ({ node: _node, children, ...props }) => {
          return (
            <div className="overflow-x-auto my-4">
              <table className="min-w-full divide-y divide-border" {...props}>
                {children}
              </table>
            </div>
          );
        },
        // Style table headers
        th: ({ node: _node, children, ...props }) => {
          return (
            <th
              className="px-4 py-2 text-left text-sm font-medium text-muted-foreground"
              {...props}
            >
              {children}
            </th>
          );
        },
        // Style table cells
        td: ({ node: _node, children, ...props }) => {
          return (
            <td className="px-4 py-2 text-sm" {...props}>
              {children}
            </td>
          );
        },
      }}
    >
      {content}
    </ReactMarkdown>
  );
};

export default MarkdownPreview;