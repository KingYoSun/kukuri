import { invokeDesktop } from './invoke/desktop';

export type LinkPreview = {
  url: string;
  source_label: string;
  title: string;
  description: string | null;
  image_data_url: string | null;
};

export type LinkPreviewUnavailableReason =
  | 'invalid_url'
  | 'blocked_target'
  | 'redirect_rejected'
  | 'too_many_redirects'
  | 'timeout'
  | 'network'
  | 'http_status'
  | 'unsupported_content'
  | 'response_too_large'
  | 'missing_metadata';

export type LinkPreviewOutcome =
  | { status: 'available'; preview: LinkPreview }
  | { status: 'unavailable'; reason: LinkPreviewUnavailableReason };

export type LinkPreviewFetcher = (url: string) => Promise<LinkPreviewOutcome>;

export function fetchLinkPreview(url: string): Promise<LinkPreviewOutcome> {
  return invokeDesktop<LinkPreviewOutcome>('fetch_link_preview', { url });
}
