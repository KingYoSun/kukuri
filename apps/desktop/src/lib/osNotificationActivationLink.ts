const PREFIX = 'kukuri://notification/?id=';
const LEGACY_PREFIX = 'kukuri://notification?id=';

/** This local navigation link carries an ID, never a target URL or account selector. */
export function parseOsNotificationActivationLink(value: string): string | null {
  // Windows adds the root slash during protocol activation. Accept that exact
  // canonical form and the previously emitted form, never arbitrary paths.
  const prefix = value.startsWith(PREFIX) ? PREFIX : LEGACY_PREFIX;
  if (!value.startsWith(prefix) || value.length > 4096) return null;
  const encodedId = value.slice(prefix.length);
  if (!encodedId || /[&#]/.test(encodedId)) return null;
  try {
    const id = decodeURIComponent(encodedId.replace(/\+/g, ' '));
    if (!id || new TextEncoder().encode(id).length > 1024 || /\p{Cc}/u.test(id)) {
      return null;
    }
    return id;
  } catch {
    return null;
  }
}
