import { useMemo, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { Notice } from '@/components/ui/notice';
import { parseMarkdown, safeExternalHref, type MarkdownBlock, type MarkdownInline } from '@/lib/markdown';
import { useExternalLinkOpener } from '@/lib/useExternalLinkOpener';
import { cn } from '@/lib/utils';

type MarkdownDocumentProps = {
  source: string;
  // 文書見出し(`#`)を置く見出しレベル。周囲の見出し階層に合わせる。
  headingLevel?: 1 | 2 | 3 | 4 | 5 | 6;
  className?: string;
};

type LinkProps = ReturnType<typeof useExternalLinkOpener>['linkProps'];

const HEADING_CLASS = [
  'text-base font-semibold text-foreground',
  'text-sm font-semibold text-foreground',
  'text-sm font-semibold text-[var(--muted-foreground)]',
];

// #1106: 運営者由来の Markdown を React 要素だけで描画する。HTML 文字列は挿入しない。
export function MarkdownDocument({ source, headingLevel = 3, className }: MarkdownDocumentProps) {
  const { t } = useTranslation('common');
  const externalLink = useExternalLinkOpener();
  const blocks = useMemo(() => parseMarkdown(source), [source]);

  return (
    <div className={cn('min-w-0 space-y-3 break-words text-sm leading-6 text-foreground [overflow-wrap:anywhere]', className)}>
      {externalLink.failed ? <Notice tone='destructive' role='alert'>{t('externalLink.failed')}</Notice> : null}
      {renderBlocks(blocks, headingLevel, externalLink.linkProps)}
    </div>
  );
}

function renderBlocks(blocks: MarkdownBlock[], headingLevel: number, linkProps: LinkProps): ReactNode[] {
  return blocks.map((block, index) => renderBlock(block, index, headingLevel, linkProps));
}

function renderBlock(block: MarkdownBlock, key: number, headingLevel: number, linkProps: LinkProps): ReactNode {
  switch (block.type) {
    case 'heading': {
      const level = Math.min(6, headingLevel + block.level - 1);
      const Tag = `h${level}` as 'h1';
      return (
        <Tag key={key} className={cn('pt-1', HEADING_CLASS[Math.min(block.level, 3) - 1])}>
          {renderInlines(block.children, linkProps)}
        </Tag>
      );
    }
    case 'paragraph':
      return <p key={key}>{renderInlines(block.children, linkProps)}</p>;
    case 'list': {
      const children = block.items.map((item, index) => (
        <li key={index} className='space-y-2 pl-1'>
          {item.length === 1 && item[0].type === 'paragraph'
            ? renderInlines(item[0].children, linkProps)
            : renderBlocks(item, headingLevel, linkProps)}
        </li>
      ));
      return block.ordered ? (
        <ol key={key} start={block.start === 1 ? undefined : block.start} className='list-decimal space-y-1 pl-6'>
          {children}
        </ol>
      ) : (
        <ul key={key} className='list-disc space-y-1 pl-6'>{children}</ul>
      );
    }
    case 'blockquote':
      return (
        <blockquote
          key={key}
          className='space-y-2 border-l-4 border-[var(--border-subtle)] pl-3 text-[var(--muted-foreground)]'
        >
          {renderBlocks(block.children, headingLevel, linkProps)}
        </blockquote>
      );
    case 'code':
      return (
        <pre
          tabIndex={0}
          key={key}
          className='overflow-x-auto rounded-[10px] border border-[var(--border-subtle)] bg-[var(--surface-panel-accent)] p-3 font-mono text-xs leading-5 [overflow-wrap:normal]'
        >
          <code>{block.value}</code>
        </pre>
      );
    case 'table':
      return (
        <div key={key} tabIndex={0} className='overflow-x-auto'>
          <table className='w-full border-collapse text-left text-xs leading-5 [overflow-wrap:normal]'>
            <thead>
              <tr>
                {block.header.map((cell, index) => (
                  <th key={index} scope='col' className='border border-[var(--border-subtle)] px-2 py-1 font-semibold'>
                    {renderInlines(cell, linkProps)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {block.rows.map((row, rowIndex) => (
                <tr key={rowIndex}>
                  {row.map((cell, index) => (
                    <td key={index} className='border border-[var(--border-subtle)] px-2 py-1 align-top'>
                      {renderInlines(cell, linkProps)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
    case 'rule':
      return <hr key={key} className='border-[var(--border-subtle)]' />;
  }
}

function renderInlines(nodes: MarkdownInline[], linkProps: LinkProps): ReactNode[] {
  return nodes.map((node, index) => renderInline(node, index, linkProps));
}

function renderInline(node: MarkdownInline, key: number, linkProps: LinkProps): ReactNode {
  switch (node.type) {
    case 'text':
      return node.value;
    case 'break':
      return <br key={key} />;
    case 'code':
      return (
        <code key={key} className='rounded-[6px] bg-[var(--surface-panel-accent)] px-1 py-0.5 font-mono text-[0.85em]'>
          {node.value}
        </code>
      );
    case 'strong':
      return <strong key={key} className='font-semibold'>{renderInlines(node.children, linkProps)}</strong>;
    case 'emphasis':
      return <em key={key}>{renderInlines(node.children, linkProps)}</em>;
    case 'link': {
      const href = safeExternalHref(node.href);
      if (!href) return <span key={key}>{renderInlines(node.children, linkProps)}</span>;
      return (
        <a
          key={key}
          href={href}
          target='_blank'
          rel='noreferrer'
          className='text-[var(--accent-foreground)] underline underline-offset-2'
          {...linkProps}
        >
          {renderInlines(node.children, linkProps)}
        </a>
      );
    }
  }
}
