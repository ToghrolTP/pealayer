/** Shared, OS-neutral rules for the server-owned preference file picker. */
export function preferenceFileAllowed(name: string, extensions: readonly string[]): boolean {
  return extensions.some(extension => name.toLowerCase().endsWith(`.${extension.toLowerCase()}`));
}

export function preferenceFileParent(path?: string): string {
  return path?.match(/^(.*[\\/])[^\\/]+$/)?.[1] ?? '';
}
