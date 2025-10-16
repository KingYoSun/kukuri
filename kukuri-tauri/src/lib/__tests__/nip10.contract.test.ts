import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { validateNip10Basic } from '../utils/nostrEventValidator';

interface Nip10Case {
  name: string;
  description?: string;
  tags: string[][];
  expected: boolean;
}

const CASES_PATH = resolve(__dirname, '../../..', 'testdata', 'nip10_contract_cases.json');

function loadCases(): Nip10Case[] {
  const raw = readFileSync(CASES_PATH, 'utf-8');
  return JSON.parse(raw) as Nip10Case[];
}

describe('NIP-10 contract cases', () => {
  const cases = loadCases();

  it('has at least one case', () => {
    expect(cases.length).toBeGreaterThan(0);
  });

  for (const testCase of cases) {
    it(`${testCase.name} (${testCase.expected ? 'valid' : 'invalid'})`, () => {
      const result = validateNip10Basic({ tags: testCase.tags });
      if (testCase.expected) {
        expect(result.ok).toBe(true);
      } else {
        expect(result.ok).toBe(false);
      }
    });
  }
});

