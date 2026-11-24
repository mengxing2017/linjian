import { afterEach, expect, it, vi } from 'vitest'
import { client } from '../src/app/api'
import { createClient } from '../src/features/identity/api/contract'
import { client as identityClient, identityApi } from '../src/features/identity/api'
import { communityApi, homeMenuApi } from '../src/features/community/api'
import { schoolsApi } from '../src/features/school-context/api'
import { operationsApi, serviceApi } from '../src/features/operations/api'
import { resourcesApi, homeMenuApi as resourcesHomeMenuApi } from '../src/features/resources/api'
import { mediaApi } from '../src/features/media/api'
import { miniappUserApi } from '../src/features/miniapp-users/api'
import { statisticsApi } from '../src/features/statistics/api'
import { feedbackApi } from '../src/features/feedback/api'

const composition = vi.hoisted(() => ({ count: 0 }))
vi.mock('../src/features/identity/api/contract', async (importOriginal) => {
  const original = await importOriginal<typeof import('../src/features/identity/api/contract')>()
  return {
    ...original,
    createClient: vi.fn((transport: Parameters<typeof original.createClient>[0]) => {
      composition.count++
      return original.createClient(transport)
    }),
  }
})
vi.mock('../src/app/transport', () => ({
  createUniTransport: vi.fn(() => vi.fn()),
}))

afterEach(() => vi.restoreAllMocks())

it('creates one application client and preserves the identity alias', () => {
  expect(vi.isMockFunction(createClient)).toBe(true)
  expect(composition.count).toBe(1)
  expect(identityClient).toBe(client)
})

it('does not expose school lists or direct school binding to the miniapp', () => {
  expect(schoolsApi).not.toHaveProperty('list')
  expect(schoolsApi).not.toHaveProperty('bind')
})

it('all feature APIs delegate to the same application client', async () => {
  const request = vi.spyOn(client, 'requestResource').mockResolvedValue(null)
  await identityApi.logout()
  await communityApi.react('school', 'post', 'like', true)
  await schoolsApi.detail('school')
  await operationsApi.event('school', {
    event_id: 'event',
    resource_id: 'resource',
    event_type: 'click',
    display_cycle: 'cycle',
    occurred_at: '2026-11-21T00:00:00Z',
  })
  await resourcesApi.click('school', 'resource')
  await homeMenuApi.settings('school', { latest_label: '最新', latest_visible: true, revision: 1 })
  await mediaApi.lease('school', 'draft', [])
  await miniappUserApi.profile()
  await statisticsApi.activity({ event_id: 'event', kind: 'feed_open', school_id: 'school' })
  await feedbackApi.submit('school', {
    feedback_type: 'other',
    content: 'feedback',
    contact: '',
    images: [],
    submission_key: 'key',
  })
  await serviceApi.options('school')
  await resourcesHomeMenuApi.public('school')
  expect(request).toHaveBeenCalledTimes(12)
  expect(request.mock.calls.map((call) => call[0])).toEqual([
    '/api/v1/auth/session',
    '/api/v1/schools/school/posts/post/reactions/like',
    '/api/v1/schools/school',
    '/api/v1/schools/school/operation-events',
    '/api/v1/schools/school/resources/resource/click',
    '/api/v1/schools/school/home-menu-settings',
    '/api/v1/schools/school/media-draft-leases/draft',
    '/api/v1/miniapp/profile',
    '/api/v1/miniapp/activity-events',
    '/api/v1/schools/school/feedback',
    '/api/v1/schools/school/operation-service-target-options',
    '/api/v1/schools/school/home-menu',
  ])
})
