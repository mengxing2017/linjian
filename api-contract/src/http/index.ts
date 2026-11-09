/** Browser compatibility facade. New platforms inject their own transport. */
import { createClient, type Cancellation, type Transport, type TransportRequest } from './client'
export * from './client'

const unauthorizedListeners = new Set<() => void>()
/** Browser session lifecycle; native clients keep their own authentication. */
export function subscribeUnauthorized(listener: () => void) {
  unauthorizedListeners.add(listener)
  return () => { unauthorizedListeners.delete(listener) }
}

async function sendBrowser(request: TransportRequest, requestBody: BodyInit | undefined = request.body) {
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
      setCsrf('')
      unauthorizedListeners.forEach((listener) => listener())
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
export const browserTransport: Transport = (request) => sendBrowser(request)
const client = createClient(browserTransport)
let browserCsrf = ''
export function setCsrf(value: string) { browserCsrf = value; client.setCsrf(value) }
/** Browser-only multipart adapter; native transports continue to receive bytes or JSON. */
export async function requestMultipart<T>(path: string, file: File, signal?: Cancellation): Promise<T> {
  const form = new FormData()
  form.append('file', file)
  const upload = createClient(request => {
    const headers = {...request.headers}
    delete headers['Content-Type']
    return sendBrowser({...request, headers}, form)
  })
  upload.setCsrf(browserCsrf)
  return upload.requestResource<T>(path, 'POST', undefined, signal)
}
export const requestDocument = client.requestDocument
export const requestCollection = client.requestCollection
// Legacy Web clients accept Blob; the protocol and other platforms only see bytes.
export async function requestResource<T>(
  path: string,
  method = 'GET',
  body?: unknown,
  signal?: Cancellation,
  headers?: Record<string, string>,
): Promise<T> {
  return client.requestResource(
    path,
    method,
    typeof Blob !== 'undefined' && body instanceof Blob ? await body.arrayBuffer() : body,
    signal,
    headers,
  )
}
export const requestJson = client.requestJson
export function getJson<T>(path: string, signal?: Cancellation): Promise<T> {
  return client.requestJson(path, 'GET', undefined, signal)
}

export const requestBinary = client.requestBinary
