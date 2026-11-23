import { ApiError, type Transport, type TransportRequest } from '../features/identity/api/contract'
import { credentials } from './credentials'
const active = new Set<{ abort(): void }>()
export function cancelRequests() {
  for (const task of active) task.abort()
  active.clear()
}
credentials.subscribe(cancelRequests)
export function trustedOrigin(value: string) {
  if (!value)
    throw new ApiError(
      0,
      'api_origin_missing',
      '未配置 VITE_LINJIAN_API_BASE_URL，请在 apps/miniapp/.env.local 中填写 API 的 HTTPS 域名后重新编译',
    )
  const origin = value.replace(/\/+$/, '')
  if (!/^https:\/\/[a-z0-9.-]+(?::\d+)?$/i.test(origin))
    throw new ApiError(
      0,
      'api_origin_invalid',
      'VITE_LINJIAN_API_BASE_URL 必须是无路径的 HTTPS origin，例如 https://api.example.com',
    )
  return origin
}
export function createUniTransport(base: string): Transport {
  // Validate only when used, so an unconfigured build can show a useful error.
  return (request: TransportRequest) =>
    new Promise((resolve, reject) => {
      const origin = trustedOrigin(base)
      const revision = credentials.revision
      if (request.signal?.aborted) {
        reject(new Error('请求已取消'))
        return
      }
      const abort = () => task.abort()
      const cleanup = () => {
        active.delete(task)
        request.signal?.removeEventListener('abort', abort)
      }
      const task = uni.request({
        url: origin + request.path,
        method: request.method as UniApp.RequestOptions['method'],
        header: {
          ...request.headers,
          ...(credentials.token ? { Authorization: `Bearer ${credentials.token}` } : {}),
        },
        data: request.body,
        responseType: request.binary ? 'arraybuffer' : 'text',
        dataType: 'text',
        timeout: 15000,
        success(response) {
          if (revision !== credentials.revision) {
            reject(new Error('会话已更新'))
            return
          }
          let body: unknown = response.data
          if (response.statusCode >= 400 && body instanceof ArrayBuffer) {
            try {
              body = decodeURIComponent(
                Array.from(
                  new Uint8Array(body),
                  (byte) => '%' + byte.toString(16).padStart(2, '0'),
                ).join(''),
              )
            } catch {
              body = undefined
            }
          }
          if (!request.binary || response.statusCode >= 400) {
            if (typeof body === 'string') {
              try {
                body = JSON.parse(body)
              } catch {
                body = undefined
              }
            }
          }
          const banned =
            !!body &&
            typeof body === 'object' &&
            'errors' in body &&
            Array.isArray(body.errors) &&
            body.errors.some(
              (e: unknown) =>
                !!e && typeof e === 'object' && 'code' in e && e.code === 'miniapp_user_banned',
            )
          if ((response.statusCode === 401 || banned) && credentials.token) {
            cleanup()
            credentials.set('')
          }
          resolve({
            status: response.statusCode,
            headers: response.header as Record<string, string>,
            body,
          })
        },
        fail(error) {
          reject(
            new ApiError(
              0,
              'network_error',
              `网络请求失败（${error.errMsg}），请检查网络、API 域名或开发者工具域名校验`,
            ),
          )
        },
        complete: cleanup,
      })
      active.add(task)
      request.signal?.addEventListener('abort', abort, { once: true })
    })
}
