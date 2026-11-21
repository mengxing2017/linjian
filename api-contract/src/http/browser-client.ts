import { sendBrowser } from './browser-transport'
import { createClient, type Cancellation, type Transport, type TransportRequest } from './client'
const unauthorizedListeners = new Set<() => void>()
/** Browser session lifecycle; native clients keep their own authentication. */
export function subscribeUnauthorized(listener: () => void) {
  unauthorizedListeners.add(listener)
  return () => {
    unauthorizedListeners.delete(listener)
  }
}

export function sendAuthenticatedBrowser(request: TransportRequest, body?: BodyInit) {
  return sendBrowser(request, body, () => {
    setCsrf('')
    unauthorizedListeners.forEach((listener) => listener())
  })
}
export const browserTransport: Transport = (request) =>
  sendAuthenticatedBrowser(request, request.body)
export const client = createClient(browserTransport)
let browserCsrf = ''
export function setCsrf(value: string) {
  browserCsrf = value
  client.setCsrf(value)
}

export const getBrowserCsrf = () => browserCsrf
export const requestDocument = client.requestDocument
export const requestCollection = client.requestCollection
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
