import type { TransportRequest } from './client'
export async function sendBrowser(
  request: TransportRequest,
  requestBody: BodyInit | undefined = request.body,
  unauthorized?: () => void,
) {
  const controller = new AbortController()
  const cancel = () => controller.abort()
  if (request.signal?.aborted) cancel()
  request.signal?.addEventListener('abort', cancel, { once: true })
  try {
    const response = await fetch(request.path, {
      method: request.method,
      credentials: 'same-origin',
      headers: request.headers,
      body: requestBody,
      signal: controller.signal,
    })
    let body: unknown
    try {
      body = request.binary && response.ok ? await response.arrayBuffer() : await response.json()
    } catch {
      body = undefined
    }
    if (response.status === 401 && !controller.signal.aborted) {
      unauthorized?.()
    }
    return {
      status: response.status,
      headers: Object.fromEntries(response.headers.entries()),
      body,
    }
  } finally {
    request.signal?.removeEventListener('abort', cancel)
  }
}
