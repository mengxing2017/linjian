import type { Client, Cancellation } from '../../http/client'
import { schoolPath } from '../schools'
export function uploadKey(): string {
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, c => { const n = Math.floor(Math.random()*16); return (c === "x" ? n : (n & 3) | 8).toString(16) })
}
export type UploadPurpose = "post_image" | "feedback_evidence"
export interface UploadedMedia {
  id: string
  purpose: string
  mime_type: string
  size_bytes: number
  width: number
  height: number
  expires_at: string
  url: string
}
export function createMediaClient(client: Client) {
  return {
    upload: (school: string, bytes: ArrayBuffer, purpose: UploadPurpose, key = uploadKey()) =>
      client.requestResource<UploadedMedia>(`${schoolPath(school)}/media?purpose=${purpose}&key=${encodeURIComponent(key)}`, 'POST', bytes),
    lease: (school: string, draft: string, asset_ids: string[]) => client.requestResource<{ id: string; lease_days: number }>(`${schoolPath(school)}/media-draft-leases/${encodeURIComponent(draft)}`, 'PUT', { asset_ids }),
    download(school: string, path: string, signal?: Cancellation) {
      const pathname=path.split('?')[0]
      const prefix = [ 'media', 'resource-media', 'resource-artwork', 'customer-images' ].map(kind=>`${schoolPath(school)}/${kind}/`).find(prefix=>pathname.startsWith(prefix)) ?? (pathname.startsWith(`${schoolPath(school)}/feedback/`) ? `${schoolPath(school)}/feedback/` : `${schoolPath(school)}/media/`)
      if (!pathname.startsWith(prefix) || !(/^[0-9a-f-]{36}$/i.test(pathname.slice(prefix.length)) || (prefix.endsWith('/customer-images/') && /^(business|join|service)$/.test(pathname.slice(prefix.length))) || new RegExp(`^${schoolPath(school)}/feedback/[0-9a-f-]{36}/images/[0-9a-f-]{36}$`, 'i').test(pathname)))
        throw new Error('图片必须属于当前学校')
      return client.requestBinary(path, signal)
    },
  }
}
