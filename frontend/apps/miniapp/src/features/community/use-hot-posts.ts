import { computed, ref, shallowRef } from 'vue'
import type { Cancellation, HotPostSummary, HotPostsMeta, HotPostsResult } from './api'

interface Scope {
  key: string
  school: string
}
interface Environment {
  scope: () => Scope | undefined
  read: (school: string, query: { limit: number }, signal: Cancellation) => Promise<HotPostsResult>
  failure: (cause: unknown) => void
  navigate: (school: string, post: string) => void
  reducedMotion?: boolean
}

function cancellation() {
  let aborted = false
  const listeners = new Set<() => void>()
  const signal: Cancellation = {
    get aborted() { return aborted },
    addEventListener: (_type, callback) => { listeners.add(callback) },
    removeEventListener: (_type, callback) => { listeners.delete(callback) },
  }
  return {
    signal,
    abort() {
      aborted = true
      listeners.forEach(listener => listener())
      listeners.clear()
    },
  }
}

export function isHotPostsHome(home: boolean, searchPage: boolean, mode: string, search: string, tag: string) {
  return home && !searchPage && mode === 'feed' && !search.trim() && !tag
}

export function createHotPosts(env: Environment) {
  const items = shallowRef<HotPostSummary[]>([])
  const meta = shallowRef<HotPostsMeta>()
  const loading = ref(false)
  const visible = ref(false)
  const userPaused = ref(env.reducedMotion ?? false)
  const touching = ref(false)
  const page = ref(0)
  const pages = computed(() => Array.from({ length: Math.ceil(items.value.length / 3) }, (_, index) => items.value.slice(index * 3, index * 3 + 3)))
  const autoplay = computed(() => visible.value && pages.value.length > 1 && !loading.value && !touching.value && !userPaused.value)
  let generation = 0
  let activeScope: Scope | undefined
  let disposed = false
  let suspended = false
  let inFlight: Promise<void> | undefined
  let controller: ReturnType<typeof cancellation> | undefined
  let queued = false
  let retryTimer: ReturnType<typeof setTimeout> | undefined
  let invalidationTimer: ReturnType<typeof setTimeout> | undefined

  function clearTimers() {
    clearTimeout(retryTimer)
    clearTimeout(invalidationTimer)
    retryTimer = undefined
    invalidationTimer = undefined
  }
  function currentScope() {
    const scope = env.scope()
    return !disposed && visible.value && scope && scope.key === activeScope?.key && scope.school === activeScope.school ? scope : undefined
  }
  function hide() {
    visible.value = false
    generation++
    controller?.abort()
    controller = undefined
    inFlight = undefined
    queued = false
    clearTimers()
    loading.value = false
    touching.value = false
    items.value = []
    meta.value = undefined
    page.value = 0
    activeScope = undefined
  }
  function refresh(): Promise<void> {
    const scope = currentScope()
    if (!scope) return Promise.resolve()
    if (inFlight) {
      queued = true
      return inFlight.then(() => inFlight)
    }
    clearTimers()
    const turn = ++generation
    const requestController = cancellation()
    controller = requestController
    const current = () => turn === generation && !!currentScope()
    loading.value = true
    const task = Promise.resolve().then(async () => {
      if (!current()) return
      try {
        const result = await env.read(scope.school, { limit: 10 }, requestController.signal)
        if (!current()) return
        const anchor = pages.value[page.value]?.[0]?.id
        items.value = result.items
        meta.value = result.meta
        const next = anchor ? result.items.findIndex(item => item.id === anchor) : -1
        page.value = next >= 0 ? Math.floor(next / 3) : 0
      } catch (cause) {
        if (!current()) return
        items.value = []
        meta.value = undefined
        page.value = 0
        if (cause && typeof cause === 'object' && (('status' in cause && [401, 403].includes(Number(cause.status))) || ('code' in cause && cause.code === 'school_unavailable'))) {
          suspended = true
          hide()
        }
        env.failure(cause)
      } finally {
        if (turn === generation) {
          inFlight = undefined
          controller = undefined
          loading.value = false
          if (current()) {
            if (queued) {
              queued = false
              void refresh()
            } else retryTimer = setTimeout(() => { void refresh() }, 60_000)
          }
        }
      }
    })
    inFlight = task
    return task
  }
  function show(revalidate = false): Promise<void> {
    if (revalidate) suspended = false
    if (disposed || suspended) return Promise.resolve()
    const scope = env.scope()
    if (visible.value && scope?.key === activeScope?.key) return inFlight ?? Promise.resolve()
    hide()
    if (!scope) return Promise.resolve()
    activeScope = scope
    visible.value = true
    return refresh()
  }
  function invalidate(event: { scope: string; school: string }) {
    const scope = currentScope()
    if (!scope || event.scope !== scope.key || event.school !== scope.school) return
    clearTimeout(invalidationTimer)
    invalidationTimer = setTimeout(() => { void refresh() }, 200)
  }
  function open(item: HotPostSummary) {
    const scope = currentScope()
    if (!scope || item.school_id !== scope.school || !items.value.some(candidate => candidate.id === item.id)) return
    userPaused.value = true
    env.navigate(scope.school, item.id)
  }
  function changePage(index: number) {
    if (Number.isInteger(index) && index >= 0 && index < pages.value.length) page.value = index
  }
  function dispose() {
    hide()
    disposed = true
  }
  return { items, meta, loading, visible, userPaused, touching, page, pages, autoplay, show, hide, refresh, invalidate, open, changePage, dispose }
}
