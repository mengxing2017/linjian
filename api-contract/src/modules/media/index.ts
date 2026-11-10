import type { Client, Cancellation } from '../../http/client'
import { schoolPath } from '../schools'
export interface UploadedMedia {
  id: string
  url: string
}
export function createMediaClient(client: Client) {
  return {
    upload: (school: string, bytes: ArrayBuffer) =>
      client.requestResource<UploadedMedia>(`${schoolPath(school)}/media`, 'POST', bytes),
    download(school: string, path: string, signal?: Cancellation) {
      const pathname=path.split('?')[0]
      const prefix = [ 'media', 'resource-media', 'resource-artwork', 'customer-images' ].map(kind=>`${schoolPath(school)}/${kind}/`).find(prefix=>pathname.startsWith(prefix)) ?? (pathname.startsWith(`${schoolPath(school)}/feedback/`) ? `${schoolPath(school)}/feedback/` : `${schoolPath(school)}/media/`)
      if (!pathname.startsWith(prefix) || !(/^[0-9a-f-]{36}$/i.test(pathname.slice(prefix.length)) || (prefix.endsWith('/customer-images/') && /^(business|join|service)$/.test(pathname.slice(prefix.length))) || new RegExp(`^${schoolPath(school)}/feedback/[0-9a-f-]{36}/images/[0-9a-f-]{36}$`, 'i').test(pathname)))
        throw new Error('图片必须属于当前学校')
      return client.requestBinary(path, signal)
    },
  }
}
