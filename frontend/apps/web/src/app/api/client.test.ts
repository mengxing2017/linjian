import { afterEach, expect, it, vi } from 'vitest'
import { client } from './index'
import { identity } from '../../features/identity/api'
import { browserClient } from '../../features/admin/workspace/api'
import { statisticsApi, statisticsPermissions } from '../../features/admin/statistics/api'
import { operationsApi, serviceApi } from '../../features/admin/operations/api'

afterEach(() => {
  client.setCsrf('')
  vi.unstubAllGlobals()
})

it('reuses the browser singleton for workspace, identity and assembled feature clients', async () => {
  expect(browserClient).toBe(client)
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(
      Response.json(
        {
          data: {
            type: 'sessions',
            id: 'current',
            attributes: { csrf: 'shared-csrf', user: null },
          },
        },
        { headers: { 'Content-Type': 'application/vnd.api+json' } },
      ),
    )
    .mockImplementation(async () =>
      Response.json({ data: null }, { headers: { 'Content-Type': 'application/vnd.api+json' } }),
    )
  vi.stubGlobal('fetch', fetcher)
  await identity.login('operator', 'password')
  await statisticsApi.report(undefined, { view: 'overview' })
  await operationsApi.customer()
  await statisticsPermissions.mine('school')
  await serviceApi.options('school')
  expect(fetcher).toHaveBeenCalledTimes(5)
  for (const [, options] of fetcher.mock.calls.slice(1)) {
    expect(options.headers['X-CSRF-Token']).toBe('shared-csrf')
    expect(options.credentials).toBe('same-origin')
  }
})
