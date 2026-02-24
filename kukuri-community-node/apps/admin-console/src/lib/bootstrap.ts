export const normalizeConnectedNode = (value: string): string => {
  const trimmed = value.trim();
  if (trimmed === '') {
    return 'unknown@unknown:0';
  }
  if (trimmed.includes('@')) {
    return trimmed;
  }
  return `${trimmed}@unknown:0`;
};
