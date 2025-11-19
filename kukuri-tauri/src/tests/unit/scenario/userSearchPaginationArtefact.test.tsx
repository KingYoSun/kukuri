import React from 'react';
import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

import { SearchErrorState } from '@/components/search/SearchErrorState';
import {
  MIN_USER_SEARCH_QUERY_LENGTH,
  detectUserSearchHelper,
  sanitizeUserSearchQuery,
  type HelperSearchDescriptor,
} from '@/hooks/useUserSearchQuery';

interface SearchErrorScenarioLog {
  timestamp: string;
  helperSearch: HelperSearchDescriptor | null;
  steps: Array<{
    step: string;
    status: string;
    retryAfterSeconds: number | null;
    sanitizedQuery: string;
    allowIncompleteActive: boolean;
    results: number;
  }>;
  searchErrorState: {
    title: string | null;
    buttonLabelBefore: string | null;
    buttonLabelAfter: string | null;
  };
}

describe('user-search pagination artefact', () => {
  it('records helper search metadata and SearchErrorState countdown for artefact export', async () => {
    const rawQuery = '@a';
    const sanitizedQuery = sanitizeUserSearchQuery(rawQuery);
    const helperSearch = detectUserSearchHelper(rawQuery);

    expect(sanitizedQuery).toBe(rawQuery.trim());
    expect(helperSearch).not.toBeNull();
    expect(helperSearch?.term).toBe('a');

    const allowIncompleteActive =
      (helperSearch?.term.length ?? 0) > 0 && (helperSearch?.term.length ?? 0) < MIN_USER_SEARCH_QUERY_LENGTH;
    const scenarioTimestamp =
      process.env.USER_SEARCH_SCENARIO_TIMESTAMP ?? formatTimestamp(new Date());

    const log: SearchErrorScenarioLog = {
      timestamp: scenarioTimestamp,
      helperSearch,
      steps: [
        {
          step: 'helperSearchDetected',
          status: 'typing',
          retryAfterSeconds: 2,
          sanitizedQuery,
          allowIncompleteActive,
          results: 0,
        },
      ],
      searchErrorState: {
        title: null,
        buttonLabelBefore: null,
        buttonLabelAfter: null,
      },
    };

    const { rerender } = render(
      <SearchErrorState
        errorKey="UserSearch.rate_limited"
        retryAfterSeconds={2}
        onRetry={() => {}}
      />,
    );

    log.searchErrorState.title =
      screen.getByText('リクエストが多すぎます')?.textContent ?? null;
    log.searchErrorState.buttonLabelBefore = screen.getByRole('button').textContent ?? null;

    rerender(
      <SearchErrorState
        errorKey="UserSearch.rate_limited"
        retryAfterSeconds={0}
        onRetry={() => {}}
      />,
    );
    log.searchErrorState.buttonLabelAfter =
      screen.getByRole('button').textContent ?? null;

    log.steps.push({
      step: 'retryAfterCleared',
      status: 'SearchErrorState.cooldownComplete',
      retryAfterSeconds: null,
      sanitizedQuery,
      allowIncompleteActive,
      results: 0,
    });

    writeSearchErrorArtefact(log);
  });
});

function formatTimestamp(date: Date): string {
  const dt = new Date(date);
  const pad = (value: number) => value.toString().padStart(2, '0');
  return `${dt.getUTCFullYear()}${pad(dt.getUTCMonth() + 1)}${pad(dt.getUTCDate())}-${pad(
    dt.getUTCHours(),
  )}${pad(dt.getUTCMinutes())}${pad(dt.getUTCSeconds())}`;
}

function writeSearchErrorArtefact(log: SearchErrorScenarioLog) {
  const repoRoot = resolve(process.cwd(), '..');
  const outputDir = join(repoRoot, 'test-results', 'user-search-pagination', 'search-error');
  mkdirSync(outputDir, { recursive: true });
  const filePath = join(outputDir, `${log.timestamp}-search-error-state.json`);
  writeFileSync(filePath, JSON.stringify(log, null, 2), 'utf8');
  const relPath = relative(repoRoot, filePath);
  // eslint-disable-next-line no-console
  console.info(`[UserSearchScenario] search error artefact saved to ${relPath}`);
}
