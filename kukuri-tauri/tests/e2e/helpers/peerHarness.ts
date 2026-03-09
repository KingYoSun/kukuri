import { browser } from '@wdio/globals';
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
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

export interface PeerHarnessCommandResult {
  command_id?: string;
  ok?: boolean;
  processed_at?: string;
  topic_id?: string | null;
  content?: string | null;
  event_id?: string | null;
  published_count?: number;
  metadata_published_count?: number;
  error?: string | null;
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

export const peerHarnessCommandDirCandidates = (peerName: string, outputGroup?: string): string[] => {
  const resolvedOutputGroup = resolveEnvString(
    outputGroup,
    process.env.KUKURI_PEER_OUTPUT_GROUP ?? 'multi-peer-e2e',
  );
  const dirName = `${peerName}.commands`;
  return [
    resolve('/app/test-results', resolvedOutputGroup, dirName),
    resolve(process.cwd(), '..', 'test-results', resolvedOutputGroup, dirName),
  ];
};

export const peerHarnessCommandResultCandidates = (
  peerName: string,
  commandId: string,
  outputGroup?: string,
): string[] =>
  peerHarnessCommandDirCandidates(peerName, outputGroup).map((dir) =>
    resolve(dir, `${commandId}.result.json`),
  );

const enqueuePeerHarnessCommand = (
  peerName: string,
  commandPayload: Record<string, unknown>,
  outputGroup?: string,
): { commandId: string } => {
  const commandId = String(commandPayload.command_id);
  const commandDirs = peerHarnessCommandDirCandidates(peerName, outputGroup);
  const targetDir = commandDirs[0];
  const commandPath = resolve(targetDir, `${commandId}.json`);
  const tempPath = resolve(targetDir, `${commandId}.json.tmp`);
  const resultPath = resolve(targetDir, `${commandId}.result.json`);

  mkdirSync(targetDir, { recursive: true });
  rmSync(commandPath, { force: true });
  rmSync(tempPath, { force: true });
  rmSync(resultPath, { force: true });
  writeFileSync(tempPath, JSON.stringify(commandPayload, null, 2), 'utf-8');
  renameSync(tempPath, commandPath);

  return { commandId };
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

export const loadPeerHarnessCommandResult = (
  peerName: string,
  commandId: string,
  outputGroup?: string,
): PeerHarnessCommandResult | null => {
  for (const candidate of peerHarnessCommandResultCandidates(peerName, commandId, outputGroup)) {
    if (!existsSync(candidate)) {
      continue;
    }
    try {
      return JSON.parse(readFileSync(candidate, 'utf-8')) as PeerHarnessCommandResult;
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

export function enqueuePeerHarnessPublishCommand(options: {
  peerName: string;
  topicId: string;
  content: string;
  replyToEventId?: string;
  outputGroup?: string;
}): { commandId: string } {
  const commandId = `publish-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
  const commandPayload = {
    command_id: commandId,
    action: 'publish_topic_event',
    topic_id: options.topicId,
    content: options.content,
    reply_to_event_id: options.replyToEventId ?? null,
  };
  return enqueuePeerHarnessCommand(options.peerName, commandPayload, options.outputGroup);
}

export async function waitForPeerHarnessCommandResult(options: {
  peerName: string;
  commandId: string;
  outputGroup?: string;
  timeoutMs?: number;
  description: string;
}): Promise<PeerHarnessCommandResult> {
  const timeoutMs = options.timeoutMs ?? 120000;
  let latest: PeerHarnessCommandResult | null = null;
  await browser.waitUntil(
    async () => {
      latest = loadPeerHarnessCommandResult(options.peerName, options.commandId, options.outputGroup);
      return latest !== null;
    },
    {
      timeout: timeoutMs,
      interval: 1000,
      timeoutMsg: `Peer harness command result did not arrive: ${options.description}`,
    },
  );

  if (!latest) {
    throw new Error(
      `Peer harness command result was not found: ${options.peerName} / ${options.commandId}`,
    );
  }

  if (!latest.ok) {
    throw new Error(
      `Peer harness command failed: ${options.description}; result=${JSON.stringify(latest)}`,
    );
  }

  return latest;
}
