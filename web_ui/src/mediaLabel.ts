/**
 * Return a compact, human-readable name for a local path or remote media URL.
 *
 * Playback always keeps the original target. Only the final displayed path
 * segment is decoded, so query parameters and URL semantics remain untouched.
 */
export const mediaBasename = (target: string, fallback: string): string => {
  const value = target.trim();
  if (!value) return fallback;

  let candidate = value;
  try {
    const parsed = new URL(value);
    const segments = parsed.pathname.split('/').filter(Boolean);
    candidate = segments[segments.length - 1] || parsed.hostname || value;
  } catch {
    const segments = value.split(/[\\/]/).filter(Boolean);
    candidate = segments[segments.length - 1] || value;
    candidate = candidate.split(/[?#]/, 1)[0] || candidate;
  }

  try {
    return decodeURIComponent(candidate) || fallback;
  } catch {
    // A malformed escape sequence must never prevent the player UI rendering.
    return candidate || fallback;
  }
};
