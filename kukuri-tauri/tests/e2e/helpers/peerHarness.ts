import { browser } from '@wdio/globals';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

export interface PeerHarnessSummary {
  peer_name?: string;
  mode?: string;
  topic_id?: string;
  status_peer_count?: number;
  status_connection?: string;
  stats?: {
    published_count?: number;
    metadata_published_count?: number;
    received_count?: number;
    recent_contents?: string[];
    last_error?: string | null;
  };
}

export interface PeerHarnessAddressSnapshot {
  peer_name?: string;
  node_addresses?: string[];
  relay_urls?: string[];
  connection_hints?: string[];
  preferred_address?: string;
}

const resolveEnvString = (value: string | undefined, fallback: string): string => {
  const trimmed = value?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : fallback;
};

export const peerHarnessSummaryCandidates = (peerName: string, outputGroup?: string): string[] => {
  const resolvedOutputGroup = resolveEnvString(
    outputGroup,
    process.env.KUKURI_PEER_OUTPUT_GROUP ?? 'multi-peer-e2e',
  );
  const fileName = `${peerName}.json`;
  return [
    resolve('/app/test-results', resolvedOutputGroup, fileName),
    resolve(process.cwd(), '..', 'test-results', resolvedOutputGroup, fileName),
  ];
};

export const peerHarnessAddressCandidates = (
  peerName: string,
  outputGroup?: string,
): string[] => {
  const resolvedOutputGroup = resolveEnvString(
    outputGroup,
    process.env.KUKURI_PEER_OUTPUT_GROUP ?? 'multi-peer-e2e',
  );
  const fileName = `${peerName}.address.json`;
  return [
    resolve('/app/test-results', resolvedOutputGroup, fileName),
    resolve(process.cwd(), '..', 'test-results', resolvedOutputGroup, fileName),
  ];
};

export const loadPeerHarnessSummary = (
  peerName: string,
  outputGroup?: string,
): PeerHarnessSummary | null => {
  for (const candidate of peerHarnessSummaryCandidates(peerName, outputGroup)) {
    if (!existsSync(candidate)) {
      continue;
    }
    try {
      return JSON.parse(readFileSync(candidate, 'utf-8')) as PeerHarnessSummary;
    } catch {
      continue;
    }
  }
  return null;
};

export const loadPeerHarnessAddressSnapshot = (
  peerName: string,
  outputGroup?: string,
): PeerHarnessAddressSnapshot | null => {
  for (const candidate of peerHarnessAddressCandidates(peerName, outputGroup)) {
    if (!existsSync(candidate)) {
      continue;
    }
    try {
      return JSON.parse(readFileSync(candidate, 'utf-8')) as PeerHarnessAddressSnapshot;
    } catch {
      continue;
    }
  }
  return null;
};

export async function waitForPeerHarnessSummary(options: {
  peerName: string;
  outputGroup?: string;
  timeoutMs?: number;
  description: string;
  predicate: (summary: PeerHarnessSummary) => boolean;
}): Promise<PeerHarnessSummary> {
  const timeoutMs = options.timeoutMs ?? 120000;
  let latest: PeerHarnessSummary | null = null;
  await browser.waitUntil(
    async () => {
      latest = loadPeerHarnessSummary(options.peerName, options.outputGroup);
      return latest ? options.predicate(latest) : false;
    },
    {
      timeout: timeoutMs,
      interval: 1000,
      timeoutMsg: `Peer harness summary did not satisfy condition: ${options.description}`,
    },
  );

  if (!latest) {
    throw new Error(`Peer harness summary was not found: ${options.peerName}`);
  }
  return latest;
}

export async function waitForPeerHarnessAddressSnapshot(options: {
  peerName: string;
  outputGroup?: string;
  timeoutMs?: number;
  description: string;
  predicate?: (snapshot: PeerHarnessAddressSnapshot) => boolean;
}): Promise<PeerHarnessAddressSnapshot> {
  const timeoutMs = options.timeoutMs ?? 120000;
  let latest: PeerHarnessAddressSnapshot | null = null;
  await browser.waitUntil(
    async () => {
      latest = loadPeerHarnessAddressSnapshot(options.peerName, options.outputGroup);
      if (!latest) {
        return false;
      }
      return options.predicate ? options.predicate(latest) : true;
    },
    {
      timeout: timeoutMs,
      interval: 1000,
      timeoutMsg: `Peer harness address snapshot did not satisfy condition: ${options.description}`,
    },
  );

  if (!latest) {
    throw new Error(`Peer harness address snapshot was not found: ${options.peerName}`);
  }
  return latest;
}
