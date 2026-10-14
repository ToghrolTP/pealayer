/** Current structured transport/RPC metadata only. Never classify OS prose. */
export function controllerErrorCaption(detail: unknown) {
  if (!detail || typeof detail !== 'object' || Array.isArray(detail)) return undefined;
  const value = detail as {kind?:unknown;transport_failed?:unknown};
  if (value.kind === 'timeout') return 'PCController request timed out';
  if (value.kind === 'connection_unavailable' || value.transport_failed === true) return 'PCController connection is unavailable';
  return undefined;
}
