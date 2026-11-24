import { afterEach, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import * as concurrency from '../src/shared/concurrency'
import * as pageWindow from '../src/shared/page-window'
import { component, findAll, mount, tap, text } from './support/vue-component'

afterEach(() => vi.unstubAllGlobals())

async function settle() {
  for (let index = 0; index < 16; index++) await Promise.resolve()
  await nextTick()
}

function setup() {
  const observers: { callback?: (result: { intersectionRatio: number }) => void; disconnect: ReturnType<typeof vi.fn> }[] = []
  const metric = vi.fn().mockResolvedValue(undefined)
  const catalog = vi.fn(async (_school: string, query: { page: number }) => ({
    server_time: new Date().toISOString(), categories: [], has_more: true,
    items: [{ id: `service-${query.page}`, title: `服务${query.page}`, subtitle: '', badge: '', image_url: `image-${query.page}` }],
  }))
  const releaseImages = vi.fn()
  vi.stubGlobal('uni', {
    pageScrollTo: vi.fn(), navigateTo: vi.fn(),
    createIntersectionObserver: () => {
      const observer = {
        callback: undefined as ((result: { intersectionRatio: number }) => void) | undefined,
        disconnect: vi.fn(),
        relativeToViewport() { return this },
        observe(_selector: string, callback: (result: { intersectionRatio: number }) => void) { this.callback = callback },
      }
      observers.push(observer)
      return observer
    },
  })
  const screen = component(new URL('../src/features/operations/Services.vue', import.meta.url), {
    './api': { serviceApi: { catalog } },
    '../school-context/context': { schoolContext: { id: 'school' }, scopeKey: () => 'scope', requireSchool: vi.fn().mockResolvedValue(true), onScopeChange: () => vi.fn() },
    '../media/files': { imagePath: vi.fn(async (_school, source) => source), releaseImages },
    '../../shared/concurrency': concurrency,
    '../../shared/page-window': pageWindow,
    '../../shared/PageWindow.vue': { default: component(new URL('../src/shared/PageWindow.vue', import.meta.url), { './page-window': pageWindow }) },
    './policy': { uuid: () => 'cycle' },
    './service-metrics': { sendServiceEvent: metric },
  })
  const mounted = mount(screen)
  async function imageLoaded(index: number) {
    const image = findAll(mounted.root, target => target.type === 'image')[index]
    ;(image.props.onLoad as () => void)()
    await nextTick()
  }
  return { mounted, catalog, metric, observers, releaseImages, imageLoaded }
}

it('keeps unexposed earlier-page observers live across appends and disconnects after one impression', async () => {
  const { mounted, metric, observers, imageLoaded } = setup()
  await mounted.exposed.value.show(); await nextTick()
  await imageLoaded(0)
  await mounted.exposed.value.loadMore(); await nextTick()
  await imageLoaded(1)
  observers[0].callback!({ intersectionRatio: 1 })
  observers[0].callback!({ intersectionRatio: 1 })
  expect(metric).toHaveBeenCalledExactlyOnceWith('school', expect.objectContaining({ resource_id: 'service-1', event_type: 'view' }))
  expect(observers[0].disconnect).toHaveBeenCalledOnce()
  mounted.unmount()
  expect(observers[1].disconnect).toHaveBeenCalledOnce()
})

it('replaces a full group, releases old resources and rejects stale exposure callbacks', async () => {
  const { mounted, metric, observers, releaseImages, catalog, imageLoaded } = setup()
  await mounted.exposed.value.show(); await nextTick()
  await imageLoaded(0)
  await mounted.exposed.value.loadMore()
  await mounted.exposed.value.loadMore(); await nextTick()
  expect(findAll(mounted.root, target => target.type === 'image')).toHaveLength(3)
  expect(text(mounted.root)).toContain('下一组')
  const releases = releaseImages.mock.calls.length
  await mounted.exposed.value.loadMore(); await nextTick()
  expect(findAll(mounted.root, target => target.type === 'image')).toHaveLength(1)
  expect(text(mounted.root)).not.toContain('服务1')
  expect(releaseImages.mock.calls.length).toBeGreaterThan(releases)
  observers[0].callback!({ intersectionRatio: 1 })
  expect(metric).not.toHaveBeenCalled()
  tap(findAll(mounted.root, target => target.type === 'button' && text(target) === '上一组')[0])
  await settle()
  expect(catalog).toHaveBeenLastCalledWith('school', expect.objectContaining({ page: 1 }))
  expect(text(mounted.root)).toContain('服务1')
  mounted.unmount()
})
