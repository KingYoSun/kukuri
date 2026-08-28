import type { AuthorSocialView, DesktopApi } from '../types';
import type { AuthorRequest } from '../types.generated';
import { invokeDesktop } from '../invoke/desktop';
import { command } from '../invoke/dispatch';

export const socialBlockApi: Pick<DesktopApi, 'blockAuthor' | 'unblockAuthor'> = {
  blockAuthor: command('blockAuthor', async (pubkey) => invokeDesktop<AuthorSocialView>(
    'block_author',
    { request: { pubkey } satisfies AuthorRequest }
  )),
  unblockAuthor: command('unblockAuthor', async (pubkey) => invokeDesktop<AuthorSocialView>(
    'unblock_author',
    { request: { pubkey } satisfies AuthorRequest }
  )),
};
