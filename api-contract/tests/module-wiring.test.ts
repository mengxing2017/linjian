import { afterEach, expect, it, vi } from 'vitest'
import { createClient, type Transport } from '../src/http/client'
import { requestMultipart, requestResource, setCsrf, subscribeUnauthorized } from '../src/http'
import { identity, workspace, product, postManagement, siteSettings } from '../src/modules/business'
import { identity as defaultIdentity } from '../src/modules/business/identity-client'
import { workspace as defaultWorkspace } from '../src/modules/business/workspace-client'
import { product as defaultProduct } from '../src/modules/product-workflows/default-client'
import { product as workflowProduct } from '../src/modules/product-workflows'
import { sourceAccess, createSourceAccessClient } from '../src/modules/source-access'
import { sourceAccess as defaultSourceAccess } from '../src/modules/source-access/default-client'

afterEach(() => {
  vi.unstubAllGlobals()
  setCsrf('')
})

it('reexports the same browser singletons through every entry', () => {
  expect(identity).toBe(defaultIdentity)
  expect(workspace).toBe(defaultWorkspace)
  expect(workspace.postManagement).toBe(postManagement)
  expect(workspace.siteSettings).toBe(siteSettings)
  expect(product).toBe(defaultProduct)
  expect(workflowProduct).toBe(defaultProduct)
  expect(sourceAccess).toBe(defaultSourceAccess)
})

it('injects source-access transport without browser APIs and keeps credentials isolated', async () => {
  const transport = vi.fn<Transport>().mockResolvedValue({
    status: 200,
    headers: { 'content-type': 'application/vnd.api+json' },
    body: { data: { type: 'source-bindings', id: 'binding', attributes: { revision: 4 } } },
  })
  const forbiddenFetch = vi.fn(() => {
    throw new Error('browser transport used')
  })
  vi.stubGlobal('fetch', forbiddenFetch)
  setCsrf('browser-csrf')
  const client = createClient(transport)
  client.setCsrf('injected-csrf')
  const source = createSourceAccessClient(client)
  const controller = new AbortController()
  await source.bindings('school 中文', controller.signal)
  expect(transport.mock.calls[0][0]).toMatchObject({
    path: `/api/v2/schools/${encodeURIComponent('school 中文')}/source-bindings`,
    signal: controller.signal,
    method: 'GET',
  })
  expect(await source.bind('school 中文', { route_id: 'route', active: true }, 3)).toEqual({
    id: 'binding',
    revision: 4,
  })
  const request = transport.mock.calls[1][0]
  expect(request.headers).toMatchObject({ 'X-CSRF-Token': 'injected-csrf', 'If-Match': '"3"' })
  expect(JSON.parse(request.body as string)).toEqual({
    data: { type: 'source-bindings', attributes: { route_id: 'route', active: true } },
  })
  expect(forbiddenFetch).not.toHaveBeenCalled()
})

it('multipart expiry clears the shared browser credential and keeps native form headers', async () => {
  const expired = vi.fn()
  const unsubscribe = subscribeUnauthorized(expired)
  const fetcher = vi.fn().mockResolvedValue(new Response('', { status: 401 }))
  vi.stubGlobal('fetch', fetcher)
  setCsrf('upload-csrf')
  try {
    await expect(
      requestMultipart('/api/v1/schools/school/post-imports', new File(['title'], 'posts.csv')),
    ).rejects.toMatchObject({ status: 401 })
    expect(expired).toHaveBeenCalledTimes(1)
    const options = fetcher.mock.calls[0][1]
    expect(options.body).toBeInstanceOf(FormData)
    expect(options.body.get('file').name).toBe('posts.csv')
    expect(options.headers['X-CSRF-Token']).toBe('upload-csrf')
    expect(options.headers).not.toHaveProperty('Content-Type')
    expect(options.credentials).toBe('same-origin')
    await expect(
      requestResource('/api/v1/profile', 'PUT', { display_name: 'Name' }),
    ).rejects.toMatchObject({ status: 401 })
    expect(fetcher.mock.calls[1][1].headers).not.toHaveProperty('X-CSRF-Token')
  } finally {
    unsubscribe()
  }
})

it('browser cancellation detaches its listener without expiring the session', async () => {
  const expired = vi.fn()
  const unsubscribe = subscribeUnauthorized(expired)
  const controller = new AbortController()
  const removeListener = vi.spyOn(controller.signal, 'removeEventListener')
  vi.stubGlobal(
    'fetch',
    vi.fn(async (_path, options) => {
      controller.abort()
      expect(options.signal.aborted).toBe(true)
      return new Response('', { status: 401 })
    }),
  )
  try {
    await expect(
      requestResource('/api/v1/auth/session', 'GET', undefined, controller.signal),
    ).rejects.toBeDefined()
    expect(removeListener).toHaveBeenCalledWith('abort', expect.any(Function))
    expect(expired).not.toHaveBeenCalled()
  } finally {
    unsubscribe()
  }
})
