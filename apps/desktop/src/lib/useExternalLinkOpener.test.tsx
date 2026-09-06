import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { invokeDesktop } from '@/lib/api/invoke/desktop';
import { isTauriRuntime } from '@/lib/releaseReadiness';
import { useExternalLinkOpener } from './useExternalLinkOpener';

vi.mock('@/lib/api/invoke/desktop', () => ({ invokeDesktop: vi.fn() }));
vi.mock('@/lib/releaseReadiness', () => ({ isTauriRuntime: vi.fn() }));
beforeEach(() => {
  vi.mocked(isTauriRuntime).mockReturnValue(true);
  vi.mocked(invokeDesktop).mockResolvedValue(undefined);
});
afterEach(() => vi.resetAllMocks());

function Fixture() {
  const external = useExternalLinkOpener();
  return <>
    <a href='https://example.test/policy' target='_blank' {...external.linkProps}>Policy</a>
    <a href='https://example.test/rights' target='_blank' {...external.linkProps}>Rights</a>
    <a href='#content'>Skip</a>
    <a href='blob:fixture' download>Download</a>
    {external.pending ? <p role='status'>Opening</p> : null}
    {external.failed ? <p role='alert'>Failed</p> : null}
  </>;
}

test('native click prevents webview navigation and passes only the explicit URL', async () => {
  render(<Fixture />);
  expect(invokeDesktop).not.toHaveBeenCalled();
  expect(fireEvent.click(screen.getByText('Policy'))).toBe(false);
  await waitFor(() => expect(invokeDesktop).toHaveBeenCalledExactlyOnceWith(
    'open_external_url', { url: 'https://example.test/policy' }
  ));
});

test('pending blocks duplicate links and failure is contained until explicit retry', async () => {
  let reject!: (reason: Error) => void;
  vi.mocked(invokeDesktop).mockReturnValueOnce(new Promise((_, rejectRequest) => { reject = rejectRequest; }));
  render(<Fixture />);
  fireEvent.click(screen.getByText('Policy'));
  fireEvent.click(screen.getByText('Policy'));
  fireEvent.click(screen.getByText('Rights'));
  expect(invokeDesktop).toHaveBeenCalledTimes(1);
  expect(screen.getByRole('status')).toHaveTextContent('Opening');
  expect(screen.getByText('Policy')).toHaveAttribute('aria-disabled', 'true');
  reject(new Error('private URL and native error must not be rendered'));
  expect(await screen.findByRole('alert')).toHaveTextContent('Failed');
  expect(screen.queryByText(/private URL/)).not.toBeInTheDocument();
  expect(invokeDesktop).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByText('Rights'));
  await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  expect(invokeDesktop).toHaveBeenCalledTimes(2);
});

test('browser, internal skip links and downloads do not invoke the OS adapter', () => {
  vi.mocked(isTauriRuntime).mockReturnValue(false);
  render(<Fixture />);
  expect(fireEvent.click(screen.getByText('Policy'))).toBe(true);
  vi.mocked(isTauriRuntime).mockReturnValue(true);
  expect(fireEvent.click(screen.getByText('Skip'))).toBe(true);
  expect(fireEvent.click(screen.getByText('Download'))).toBe(true);
  expect(invokeDesktop).not.toHaveBeenCalled();
});

test('native middle click opens once while right click preserves its context menu', async () => {
  render(<Fixture />);
  const link = screen.getByText('Policy');
  expect(fireEvent(link, new MouseEvent('auxclick', { bubbles: true, cancelable: true, button: 2 }))).toBe(true);
  expect(invokeDesktop).not.toHaveBeenCalled();
  expect(fireEvent(link, new MouseEvent('auxclick', { bubbles: true, cancelable: true, button: 1 }))).toBe(false);
  await waitFor(() => expect(invokeDesktop).toHaveBeenCalledTimes(1));
});
