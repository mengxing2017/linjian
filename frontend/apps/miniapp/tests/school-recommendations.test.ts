import { afterEach, expect, it, vi } from 'vitest'
import { createSchoolRecommendations } from '../src/features/school-context/school-recommendations'
import { LocationFailure, location, normalizeCoordinates } from '../src/features/school-context/location'

const coordinates = normalizeCoordinates(23.12345678, 113.12345678)
function setup() {
  const dependencies = { privacyRequired: vi.fn().mockResolvedValue(false), authorize: vi.fn().mockResolvedValue(undefined), locate: vi.fn().mockResolvedValue(coordinates), request: vi.fn().mockResolvedValue([]), failure: vi.fn() }
  return { dependencies, controller: createSchoolRecommendations(dependencies) }
}
afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals() })
it('normalizes native precision without accepting invalid coordinates', () => {
  expect(coordinates).toEqual({ latitude: 23.123457, longitude: 113.123457, coordinate_system: 'gcj02' })
  for (const [latitude, longitude] of [[NaN, 0], [0, Infinity], [90.0000001, 0], [0, -180.0000001]]) expect(() => normalizeCoordinates(latitude, longitude)).toThrow()
  expect(normalizeCoordinates(0, 0).latitude).toBe(0)
})
it('does nothing before user action and stops before location when privacy is not agreed', async () => {
  const { dependencies, controller } = setup()
  expect(dependencies.locate).not.toHaveBeenCalled()
  dependencies.privacyRequired.mockResolvedValue(true)
  await controller.recommend()
  expect(controller.state.value).toBe('privacy')
  expect(dependencies.authorize).not.toHaveBeenCalled()
  expect(dependencies.request).not.toHaveBeenCalled()
  controller.decline()
  expect(controller.state.value).toBe('privacy-denied')
})
it('does not send a location when authorization is refused', async () => {
  const { dependencies, controller } = setup()
  dependencies.authorize.mockRejectedValue(new LocationFailure('denied', '未授权位置'))
  await controller.recommend()
  expect(controller.state.value).toBe('denied')
  expect(dependencies.locate).not.toHaveBeenCalled()
  expect(dependencies.request).not.toHaveBeenCalled()
})
it('displays read-only candidates without selecting or binding and treats an empty result as success', async () => {
  const { dependencies, controller } = setup()
  const rows = [{ id: 'school', name: '推荐学校', distance_m: 10 }]
  dependencies.request.mockResolvedValue(rows)
  await controller.recommend()
  expect(controller.state.value).toBe('success')
  expect(controller.items.value).toEqual(rows)
  expect(dependencies.request).toHaveBeenCalledWith(coordinates, expect.objectContaining({ aborted: false }))
  dependencies.request.mockResolvedValue([])
  await controller.recommend()
  expect(controller.state.value).toBe('empty')
})
it('ignores late results after manual searching, hiding, or scope invalidation', async () => {
  const { dependencies, controller } = setup()
  let finish!: (value: typeof coordinates) => void
  dependencies.locate.mockImplementation(() => new Promise(resolve => { finish = resolve }))
  const pending = controller.recommend()
  await vi.waitFor(() => expect(controller.state.value).toBe('locating'))
  controller.clear()
  finish(coordinates)
  await pending
  expect(controller.state.value).toBe('idle')
  expect(dependencies.request).not.toHaveBeenCalled()
})
it('deduplicates clicks and ignores location callbacks after a timeout', async () => {
  vi.useFakeTimers()
  const { dependencies, controller } = setup()
  let finish!: (value: typeof coordinates) => void
  dependencies.locate.mockImplementation(() => new Promise(resolve => { finish = resolve }))
  const pending = controller.recommend()
  await vi.advanceTimersByTimeAsync(1)
  await controller.recommend()
  expect(dependencies.locate).toHaveBeenCalledTimes(1)
  await vi.advanceTimersByTimeAsync(10000)
  await pending
  expect(controller.state.value).toBe('failed')
  finish(coordinates)
  await vi.advanceTimersByTimeAsync(1)
  expect(dependencies.request).not.toHaveBeenCalled()
})
it('aborts a running recommendation request when manual selection invalidates it', async () => {
  const { dependencies, controller } = setup()
  let finish!: (value: unknown[]) => void
  dependencies.request.mockImplementation(() => new Promise(resolve => { finish = resolve }))
  const pending = controller.recommend()
  await vi.waitFor(() => expect(controller.state.value).toBe('loading'))
  const signal = dependencies.request.mock.calls[0][1]
  const aborted = vi.fn()
  signal.addEventListener('abort', aborted)
  controller.clear()
  await pending
  expect(aborted).toHaveBeenCalledTimes(1)
  expect(signal.aborted).toBe(true)
  finish([{ id: 'late' }])
  await Promise.resolve()
  expect(controller.items.value).toEqual([])
})
it('does not repeatedly authorize when location permission was previously refused', async () => {
  const authorize = vi.fn(), getLocation = vi.fn()
  vi.stubGlobal('uni', { getSetting: vi.fn().mockResolvedValue({ authSetting: { 'scope.userLocation': false } }), authorize, getLocation })
  await expect(location.authorize()).rejects.toMatchObject({ reason: 'denied' })
  expect(authorize).not.toHaveBeenCalled()
  expect(getLocation).not.toHaveBeenCalled()
})
it('skips a redundant permission prompt when already authorized', async () => {
  const authorize = vi.fn()
  vi.stubGlobal('uni', { getSetting: vi.fn().mockResolvedValue({ authSetting: { 'scope.userLocation': true } }), authorize })
  await location.authorize()
  expect(authorize).not.toHaveBeenCalled()
})
