export type DesktopDistribution = 'direct' | 'microsoft-store';

export function desktopDistributionFromEnvironment(value?: string): DesktopDistribution {
  return value === 'microsoft-store' ? 'microsoft-store' : 'direct';
}

export const DESKTOP_DISTRIBUTION = desktopDistributionFromEnvironment(
  import.meta.env.VITE_KUKURI_DISTRIBUTION
);

export function usesSelfManagedUpdater(
  distribution: DesktopDistribution = DESKTOP_DISTRIBUTION
): boolean {
  return distribution === 'direct';
}
