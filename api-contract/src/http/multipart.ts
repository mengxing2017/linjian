import { getBrowserCsrf, sendAuthenticatedBrowser } from './browser-client'
import { createClient, type Cancellation } from './client'
export async function requestMultipart<T>(
  path: string,
  file: File,
  signal?: Cancellation,
): Promise<T> {
  const form = new FormData()
  form.append('file', file)
  const upload = createClient((request) => {
    const headers = { ...request.headers }
    delete headers['Content-Type']
    return sendAuthenticatedBrowser({ ...request, headers }, form)
  })
  upload.setCsrf(getBrowserCsrf())
  return upload.requestResource<T>(path, 'POST', undefined, signal)
}
