import { normalizeConnectedNode } from './bootstrap';
import { asRecord, findServiceByName } from './config';
import type { ServiceInfo } from './types';

export type RelayRuntimeSnapshot = {
  wsConnections: number | null;
  bootstrapNodes: string[];
};

const asFiniteInteger = (value: unknown): number | null => {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return Math.max(0, Math.round(value));
  }
  return null;
};

const parseBootstrapNodes = (value: unknown): string[] => {
  if (!Array.isArray(value)) {
    return [];
  }

  return Array.from(
    new Set(
      value
        .filter((node): node is string => typeof node === 'string')
        .map((node) => node.trim())
        .filter((node) => node !== '')
        .map(normalizeConnectedNode)
    )
  ).sort();
};

export const parseRelayRuntimeSnapshot = (
  services: ServiceInfo[] | undefined
): RelayRuntimeSnapshot => {
  const relayService = findServiceByName(services, 'relay');
  const details = asRecord(relayService?.health?.details);
  const authTransition = asRecord(details?.auth_transition);
  const p2pRuntime = asRecord(details?.p2p_runtime);

  return {
    wsConnections: asFiniteInteger(authTransition?.ws_connections),
    bootstrapNodes: parseBootstrapNodes(p2pRuntime?.bootstrap_nodes)
  };
};
