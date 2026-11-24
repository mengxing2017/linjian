import { afterEach, expect, it, vi } from 'vitest'
import { h, nextTick } from 'vue'
import { component, findAll, mount, tap, text } from './support/vue-component'

afterEach(() => vi.unstubAllGlobals())

function setup() {
  const callbacks = new Set<() => void>()
  const imagePath = vi.fn().mockResolvedValue('/private/image')
  const clearScope = vi.fn(() => callbacks.forEach((callback) => callback()))
  const previewImage = vi.fn()
  vi.stubGlobal('uni', { previewImage })
  const image = component(new URL('../src/features/media/PrivateImage.vue', import.meta.url), {
    './files': { imagePath, releaseImages: vi.fn() },
    '../school-context/context': {
      onScopeChange: (callback: () => void) => {
        callbacks.add(callback)
        return () => callbacks.delete(callback)
      },
      clearScope,
    },
  })
  return { image, imagePath, callbacks, clearScope, previewImage }
}

async function settle() {
  await Promise.resolve()
  await nextTick()
}

it('keeps detail widthFix by default, uses a bounded feed variant and stops preview taps from opening a post', async () => {
  const { image, previewImage } = setup()
  const open = vi.fn()
  const mounted = mount({
    render: () =>
      h('view', { onTap: open }, [
        h(image, { school: 'school', source: 'portrait', variant: 'feed' }),
      ]),
  })
  await settle()
  const rendered = findAll(mounted.root, (target) => target.type === 'image')[0]
  expect(rendered.props.mode).toBe('aspectFill')
  expect(rendered.props.class).toContain('feed-image')
  tap(rendered)
  expect(previewImage).toHaveBeenCalledWith({ urls: ['/private/image'] })
  expect(open).not.toHaveBeenCalled()
  mounted.unmount()
  const detail = mount(image, { school: 'school', source: 'portrait' })
  await settle()
  expect(findAll(detail.root, (target) => target.type === 'image')[0].props.mode).toBe('widthFix')
  detail.unmount()
})

it('uses the feed slot during load and failure, retries download/decode failures, and cleans up on unmount', async () => {
  const { image, imagePath, callbacks } = setup()
  imagePath.mockRejectedValueOnce(new Error('offline'))
  const mounted = mount(image, { school: 'school', source: 'portrait', variant: 'feed' })
  expect(
    findAll(mounted.root, (target) => String(target.props.class).includes('feed-placeholder')),
  ).toHaveLength(1)
  await settle()
  expect(text(mounted.root)).toContain('图片加载失败')
  tap(findAll(mounted.root, (target) => target.props.role === 'button')[0])
  await settle()
  const rendered = findAll(mounted.root, (target) => target.type === 'image')[0]
  ;(rendered.props.onError as () => void)()
  await nextTick()
  expect(
    findAll(mounted.root, (target) => String(target.props.class).includes('feed-placeholder')),
  ).toHaveLength(1)
  tap(findAll(mounted.root, (target) => target.props.role === 'button')[0])
  await settle()
  expect(imagePath).toHaveBeenCalledTimes(3)
  expect(findAll(mounted.root, (target) => target.type === 'image')).toHaveLength(1)
  mounted.unmount()
  expect(callbacks.size).toBe(0)
})

it('ignores stale image successes and security errors after switching schools', async () => {
  const { image, imagePath, callbacks, clearScope } = setup()
  let reject!: (cause: unknown) => void
  imagePath.mockReturnValueOnce(
    new Promise((_resolve, fail) => {
      reject = fail
    }),
  )
  const mounted = mount(image, { school: 'first', source: 'old', variant: 'feed' })
  callbacks.forEach((callback) => callback())
  mounted.props.school = 'second'
  mounted.props.source = 'fresh'
  await settle()
  reject({ status: 403 })
  await settle()
  expect(clearScope).not.toHaveBeenCalled()
  expect(findAll(mounted.root, (target) => target.type === 'image')[0].props.src).toBe(
    '/private/image',
  )
  let resolve!: (path: string) => void
  imagePath.mockReturnValueOnce(
    new Promise((done) => {
      resolve = done
    }),
  )
  mounted.props.source = 'late'
  await nextTick()
  callbacks.forEach((callback) => callback())
  resolve('/wrong-school')
  await settle()
  expect(findAll(mounted.root, (target) => target.type === 'image')).toHaveLength(0)
  mounted.unmount()
})

it('still clears the current school context on an active image permission failure', async () => {
  const { image, imagePath, clearScope } = setup()
  imagePath.mockRejectedValueOnce({ status: 403 })
  const mounted = mount(image, { school: 'school', source: 'forbidden', variant: 'feed' })
  await settle()
  expect(clearScope).toHaveBeenCalledOnce()
  expect(findAll(mounted.root, (target) => target.type === 'image')).toHaveLength(0)
  mounted.unmount()
})

it('defers private feed downloads until near the viewport and disconnects the visibility observer', async () => {
  const { image, imagePath } = setup()
  const observe = vi.fn(), disconnect = vi.fn()
  const observer = { relativeToViewport: vi.fn(() => observer), observe, disconnect }
  Object.assign(uni, { createIntersectionObserver: vi.fn(() => observer) })
  const mounted = mount(image, { school: 'school', source: 'offscreen', variant: 'feed' })
  await settle()
  expect(imagePath).not.toHaveBeenCalled()
  const visible = observe.mock.calls[0][1] as (value: { intersectionRatio: number }) => void
  visible({ intersectionRatio: 0 })
  expect(imagePath).not.toHaveBeenCalled()
  visible({ intersectionRatio: 1 })
  await settle()
  expect(imagePath).toHaveBeenCalledOnce()
  expect(disconnect).toHaveBeenCalledOnce()
  visible({ intersectionRatio: 1 })
  expect(imagePath).toHaveBeenCalledOnce()
  mounted.unmount()
})
