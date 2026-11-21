import type { Cancellation, Client } from '../../http/client'
import { schoolPath } from '../schools/path'
import type { UploadPurpose, UploadedMedia } from './types'
import { uploadKey } from './upload-key'

export function createMediaClient(client: Client) {
  return {
    upload: (school: string, bytes: ArrayBuffer, purpose: UploadPurpose, key = uploadKey()) =>
      client.requestResource<UploadedMedia>(
        `${schoolPath(school)}/media?purpose=${purpose}&key=${encodeURIComponent(key)}`,
        'POST',
        bytes,
      ),
    lease: (school: string, draft: string, asset_ids: string[]) =>
      client.requestResource<{ id: string; lease_days: number }>(
        `${schoolPath(school)}/media-draft-leases/${encodeURIComponent(draft)}`,
        'PUT',
        { asset_ids },
      ),
    download(school: string, path: string, signal?: Cancellation) {
      const pathname = path.split('?')[0]
      const prefix =
        ['media', 'resource-media', 'resource-artwork', 'customer-images']
          .map((kind) => `${schoolPath(school)}/${kind}/`)
          .find((prefix) => pathname.startsWith(prefix)) ??
        (pathname.startsWith(`${schoolPath(school)}/feedback/`)
          ? `${schoolPath(school)}/feedback/`
          : `${schoolPath(school)}/media/`)
      if (
        !pathname.startsWith(prefix) ||
        !(
          /^[0-9a-f-]{36}$/i.test(pathname.slice(prefix.length)) ||
          (prefix.endsWith('/customer-images/') &&
            /^(business|join|service)$/.test(pathname.slice(prefix.length))) ||
          new RegExp(
            `^${schoolPath(school)}/feedback/[0-9a-f-]{36}/images/[0-9a-f-]{36}$`,
            'i',
          ).test(pathname)
        )
      )
        throw new Error('图片必须属于当前学校')
      return client.requestBinary(path, signal)
    },
  }
}
