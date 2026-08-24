import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const APP_DIR = process.cwd();

function productionSource(root: string): string {
  return readdirSync(root, { withFileTypes: true })
    .flatMap((entry) => {
      const path = join(root, entry.name);
      if (entry.isDirectory()) return productionSource(path);
      if (
        !entry.name.match(/\.(?:ts|tsx)$/) ||
        entry.name.includes('.test.') ||
        entry.name.includes('.stories.')
      ) {
        return [];
      }
      return [readFileSync(path, 'utf8')];
    })
    .join('\n');
}

describe('Issue #767 legacy projection hygiene', () => {
  const shellSource = productionSource(resolve(APP_DIR, 'src/shell'));

  it('does not restore removed synchronization, global composer, or duplicate cache symbols', () => {
    for (const symbol of [
      'useDesktopShellColumnSynchronization',
      'composeDialogOpen',
      'openNewPostDialog',
      'publicTimelinesByTopic',
      'publicTimelineNextCursorByTopic',
      'livePanelStateByTopic',
      'gamePanelStateByTopic',
    ]) {
      expect(shellSource, symbol).not.toContain(symbol);
    }
  });

  it('keeps removed component files and selectors absent', () => {
    expect(
      existsSync(resolve(APP_DIR, 'src/components/core/TimelineWorkspaceHeader.tsx'))
    ).toBe(false);
    const styles = [
      'shell-scoped-overrides.css',
      'shell-phase1-part2.css',
      'shell-phase1-part3.css',
    ]
      .map((name) => readFileSync(resolve(APP_DIR, 'src/styles', name), 'utf8'))
      .join('\n');
    expect(styles).not.toContain('.shell-phase1 .grid');
    expect(styles).not.toContain('.shell-workspace-header-card');
    expect(styles).not.toContain('.shell-status-badges');
    expect(styles).not.toContain('.shell-compose-dialog');
  });
});
