// Browser and native clients use the same ordered, bounded transfer contract.
import { newCueId } from './cueAuthoring.ts';
export type AssetDestination = { surface: 'player' | 'library' } | { surface: 'timeline'; start_time_ms: number; track_key?: string };
export interface ImportedEffect {
  id: string; reference: string; name: string; icon: string; category: string; description: string;
  kind: 'audio' | 'sequence' | 'strip-stream'; program: unknown; color: string; default_fps: number; default_pixels: number; duration_ms: number;
}
export interface AssetReview { id: string; name: string; kind: 'scene' | 'timeline' | 'effect'; destination: AssetDestination; summary: string[]; effect?: ImportedEffect }
export interface AssetImportState { busy: boolean; reviews: AssetReview[] }
type SendCommand = (command: string, payload?: Record<string, unknown>) => Promise<boolean>;
export const externalFiles = (transfer: DataTransfer) => transfer.types.includes('Files');
export function assetDropDestination(target: EventTarget | null, clientX: number, fallback: AssetDestination): AssetDestination {
  const lane = target instanceof Element ? target.closest<HTMLElement>('[data-asset-lane]') : null;
  const duration = Number(lane?.dataset.assetDuration);
  if (!lane || !Number.isSafeInteger(duration) || duration < 1) return fallback;
  const rect=lane.getBoundingClientRect();
  return {surface:'timeline',track_key:lane.dataset.assetLane,start_time_ms:Math.round(Math.max(0,Math.min(1,(clientX-rect.left)/Math.max(1,rect.width)))*duration)};
}

export async function importAssetFiles(files: File[], destination: AssetDestination, base: string, sendCmd: SendCommand, progress?: (percent: number) => void, signal?: AbortSignal) {
  const receipt = await uploadAssetFiles(files, base, progress, signal);
  try {
    // Retain BEFORE submission: a lost command acknowledgement must not let
    // cleanup delete a file already queued for import on the authority.
    await receipt.retain();
    if (!await sendCmd('import_assets', { request: { paths: receipt.paths, destination, owner_id: receipt.owner_id } })) throw new Error('Asset import was not accepted');
  } catch(error) { await receipt.cancel(); throw error; }
}

/** Also used by the existing sound-effect source input: upload, then set the
 * field only. It must not create a second SFX or load the main player. */
export async function uploadAssetFiles(files: File[], base: string, progress?: (percent: number) => void, signal?: AbortSignal) {
  if (!files.length || files.length > 64) throw new Error('Choose 1–64 asset files');
  if (files.some(file => !file.size || file.size > 8 * 1024 ** 3)) throw new Error('Each asset must be between 1 byte and 8 GiB');
  if (new Set(files.map(file => file.name.toLocaleLowerCase())).size !== files.length) throw new Error('Files in a single drop must have distinct names');
  const batch = newCueId();
  let owner: string | undefined;
  const pinned = (path: string) => owner ? `${path}${path.includes('?') ? '&' : '?'}owner_id=${encodeURIComponent(owner)}` : path;
  const cancel = async () => { await fetch(`${base}${pinned(`/api/assets/upload/batch?batch_id=${batch}`)}`, { method: 'DELETE' }).catch(() => undefined); };
  const paths: string[] = [];
  const total = files.reduce((sum, file) => sum + file.size, 0);
  let sent = 0;
  const request = async (path: string, init: RequestInit) => {
    const response = await fetch(`${base}${path}`, { ...init, signal });
    const value = await response.json();
    if (!response.ok) throw new Error(value.error || `Asset transfer failed (${response.status})`);
    return value;
  };
  try { for (const file of files) {
    const begin = await request('/api/assets/upload/begin', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ batch_id: batch, name: file.name, size: file.size }) });
    if (typeof begin.id !== 'string' || typeof begin.owner_id !== 'string' || !begin.owner_id || !Number.isInteger(begin.chunk_bytes) || begin.chunk_bytes < 1 || begin.chunk_bytes > 512 * 1024) throw new Error('Invalid asset transfer contract');
    if (owner && owner !== begin.owner_id) throw new Error('Asset session owner changed; retry the import');
    owner = begin.owner_id;
    const id = encodeURIComponent(begin.id);
    try {
      for (let offset = 0; offset < file.size; offset += begin.chunk_bytes) {
        const chunk = file.slice(offset, offset + begin.chunk_bytes);
        const receipt = await request(pinned(`/api/assets/upload/chunk?id=${id}&offset=${offset}`), { method: 'PUT', headers: { 'Content-Type': 'application/octet-stream' }, body: chunk });
        if (receipt.offset !== offset + chunk.size) throw new Error('Asset transfer offset mismatch');
        sent += chunk.size; progress?.(Math.round(sent * 100 / total));
      }
      const receipt = await request(pinned(`/api/assets/upload/finish?id=${id}`), { method: 'POST' });
      if (typeof receipt.path !== 'string' || !receipt.path || receipt.owner_id !== owner) throw new Error('Asset transfer did not return a stored file from the selected owner');
      paths.push(receipt.path);
    } catch (error) {
      // A fresh request can cancel a transfer even after the main signal aborts.
      await fetch(`${base}${pinned(`/api/assets/upload?id=${id}`)}`, { method: 'DELETE' }).catch(() => undefined);
      throw error;
    }
  } } catch (error) { await cancel(); throw error; }
  const retain = async () => { await request(pinned('/api/assets/upload/retain'),{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({paths})}); };
  return { paths, owner_id: owner!, cancel, retain };
}
