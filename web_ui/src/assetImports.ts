// Browser and native clients use the same ordered, bounded transfer contract.
import { newCueId } from './cueAuthoring.ts';
export type AssetDestination = { surface: 'player' | 'library' } | { surface: 'timeline'; start_time_ms: number; track_key?: string };
export interface ImportedEffect {
  id: string; reference: string; name: string; icon: string; category: string; description: string;
  kind: 'audio' | 'sequence' | 'strip-stream'; program: unknown; color: string; default_fps: number; default_pixels: number; duration_ms: number;
}
export interface AssetReview { id: string; name: string; kind: 'scene' | 'timeline' | 'effect'; destination: AssetDestination; effect?: ImportedEffect }
export interface AssetImportState { busy: boolean; reviews: AssetReview[] }
type SendCommand = (command: string, payload?: Record<string, unknown>) => Promise<boolean>;
export const externalFiles = (transfer: DataTransfer) => transfer.types.includes('Files');

export async function importAssetFiles(files: File[], destination: AssetDestination, base: string, sendCmd: SendCommand, progress?: (percent: number) => void, signal?: AbortSignal) {
  if (!files.length || files.length > 64) throw new Error('Choose 1–64 asset files');
  if (files.some(file => !file.size || file.size > 8 * 1024 ** 3)) throw new Error('Each asset must be between 1 byte and 8 GiB');
  const batch = newCueId();
  const paths: string[] = [];
  const total = files.reduce((sum, file) => sum + file.size, 0);
  let sent = 0;
  const request = async (path: string, init: RequestInit) => {
    const response = await fetch(`${base}${path}`, { ...init, signal });
    const value = await response.json();
    if (!response.ok) throw new Error(value.error || `Asset transfer failed (${response.status})`);
    return value;
  };
  for (const file of files) {
    const begin = await request('/api/assets/upload/begin', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ batch_id: batch, name: file.name, size: file.size }) });
    if (typeof begin.id !== 'string' || !Number.isInteger(begin.chunk_bytes) || begin.chunk_bytes < 1 || begin.chunk_bytes > 512 * 1024) throw new Error('Invalid asset transfer contract');
    const id = encodeURIComponent(begin.id);
    try {
      for (let offset = 0; offset < file.size; offset += begin.chunk_bytes) {
        const chunk = file.slice(offset, offset + begin.chunk_bytes);
        const receipt = await request(`/api/assets/upload/chunk?id=${id}&offset=${offset}`, { method: 'PUT', headers: { 'Content-Type': 'application/octet-stream' }, body: chunk });
        if (receipt.offset !== offset + chunk.size) throw new Error('Asset transfer offset mismatch');
        sent += chunk.size; progress?.(Math.round(sent * 100 / total));
      }
      const receipt = await request(`/api/assets/upload/finish?id=${id}`, { method: 'POST' });
      if (typeof receipt.path !== 'string' || !receipt.path) throw new Error('Asset transfer did not return a stored file');
      paths.push(receipt.path);
    } catch (error) {
      // A fresh request can cancel a transfer even after the main signal aborts.
      await fetch(`${base}/api/assets/upload?id=${id}`, { method: 'DELETE' }).catch(() => undefined);
      throw error;
    }
  }
  if (!await sendCmd('import_assets', { request: { paths, destination } })) throw new Error('Asset import was not accepted');
}
