import type { ServiceInfo } from './types';

export const asRecord = (value: unknown): Record<string, unknown> | null => {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    return null;
  }
  return value as Record<string, unknown>;
};

export const findServiceByName = (
  services: ServiceInfo[] | undefined,
  serviceName: string
): ServiceInfo | null => {
  if (!services) {
    return null;
  }
  return services.find((service) => service.service === serviceName) ?? null;
};
