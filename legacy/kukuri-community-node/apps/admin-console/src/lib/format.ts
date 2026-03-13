export const formatTimestamp = (value?: number | null) => {
  if (!value) {
    return '—';
  }
  return new Date(value * 1000).toLocaleString();
};

export const formatJson = (value: unknown) => JSON.stringify(value, null, 2);
