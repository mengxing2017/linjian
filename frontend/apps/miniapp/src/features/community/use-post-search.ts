import { ref, shallowRef } from 'vue'
import { mergePageWindow } from '../../shared/page-window'
import { feedPost } from './post-feed-card'
import { mapConcurrent } from '../../shared/concurrency'
import {
  normalizePostSearch,
  type communityApi,
  type CommunityPost,
  type PostInteractionState,
  type Cancellation,
} from './api'
import { acknowledgePost, invalidatedPosts } from './post-invalidations'

interface Scope {
  key: string
  school: string
  user: string
}
interface Environment {
  api: Pick<typeof communityApi, 'posts' | 'interactionState'>
  scope: () => Scope | undefined
  ensure: () => Promise<boolean>
  failure: (cause: unknown) => string
}

function cancellation() {
  let aborted = false
  const listeners = new Set<() => void>()
  const signal: Cancellation = {
    get aborted() {
      return aborted
    },
    addEventListener: (_type, callback) => {
      listeners.add(callback)
    },
    removeEventListener: (_type, callback) => {
      listeners.delete(callback)
    },
  }
  return {
    signal,
    abort: () => {
      aborted = true
      listeners.forEach((listener) => listener())
      listeners.clear()
    },
  }
}

export function createPostSearch(env: Environment) {
  const draftKeyword = ref('')
  const submittedKeyword = ref('')
  const posts = shallowRef<CommunityPost[]>([])
  const page = ref(0)
  const hasMore = ref(false)
  const loading = ref(false)
  const loadingMore = ref(false)
  const error = ref('')
  const moreError = ref('')
  const inputError = ref('')
  const visible = ref(false)
  const ready = ref(false)
  let revision = 0
  let showTurn = 0
  let disposed = false
  let activeScope: Scope | undefined
  let knownUser = env.scope()?.user
  let controller: ReturnType<typeof cancellation> | undefined
  let inFlight: Promise<void> | undefined
  let syncing: Promise<void> | undefined

  function sameScope(scope: Scope) {
    const current = env.scope()
    return (
      !!current &&
      current.key === scope.key &&
      current.school === scope.school &&
      current.user === scope.user
    )
  }
  function snapshot() {
    const turn = revision
    const scope = activeScope
    return () => !disposed && turn === revision && !!scope && sameScope(scope)
  }
  function reset(clearDraft = false) {
    revision++
    controller?.abort()
    controller = undefined
    inFlight = undefined
    activeScope = undefined
    submittedKeyword.value = ''
    posts.value = []
    page.value = 0
    hasMore.value = false
    loading.value = false
    loadingMore.value = false
    error.value = ''
    moreError.value = ''
    inputError.value = ''
    if (clearDraft) draftKeyword.value = ''
  }
  function scopeChanged() {
    const user = env.scope()?.user
    reset(user !== knownUser)
    knownUser = user
    ready.value = false
  }
  async function read(next: number) {
    if (!activeScope || !sameScope(activeScope) || !submittedKeyword.value || disposed) return
    const current = snapshot()
    const scope = activeScope
    const keyword = submittedKeyword.value
    let succeeded = false
    const requestController = cancellation()
    controller = requestController
    if (next === 1) {
      loading.value = true
      error.value = ''
    } else {
      loadingMore.value = true
      moreError.value = ''
    }
    try {
      const response = await env.api.posts(
        scope.school,
        { search: keyword, page: next },
        requestController.signal,
      )
      if (!current()) return
      if (!response.page || response.page.number !== next)
        throw new Error('搜索分页信息异常，请重试')
      posts.value = mergePageWindow(posts.value, page.value, response.items.map(feedPost), next)
      page.value = next
      hasMore.value = response.page.has_more
      succeeded = true
    } catch (cause) {
      if (!current()) return
      const message = env.failure(cause)
      if (current()) {
        if (next === 1) error.value = message
        else moreError.value = message
      }
    } finally {
      if (current()) {
        loading.value = false
        loadingMore.value = false
        controller = undefined
        if (succeeded && visible.value) void Promise.resolve().then(sync)
      }
    }
  }
  function start(keyword: string) {
    const scope = env.scope()
    if (!scope || !ready.value || disposed) return Promise.resolve()
    reset()
    activeScope = { ...scope }
    knownUser = scope.user
    submittedKeyword.value = keyword
    const request = read(1).finally(() => {
      if (inFlight === request) inFlight = undefined
    })
    inFlight = request
    return request
  }
  function submit() {
    inputError.value = ''
    let keyword: string
    try {
      keyword = normalizePostSearch(draftKeyword.value)
    } catch (cause) {
      inputError.value = cause instanceof Error ? cause.message : '搜索关键词不合法'
      return Promise.resolve()
    }
    if (!keyword) {
      inputError.value = '请输入搜索关键词'
      return Promise.resolve()
    }
    if (
      loading.value &&
      keyword === submittedKeyword.value &&
      activeScope &&
      sameScope(activeScope)
    )
      return inFlight ?? Promise.resolve()
    return start(keyword)
  }
  function refresh() {
    return submittedKeyword.value ? start(submittedKeyword.value) : Promise.resolve()
  }
  function loadMore() {
    if (!ready.value || !hasMore.value || !page.value || loading.value || loadingMore.value)
      return Promise.resolve()
    return read(page.value + 1)
  }
  function loadPage(target: number) {
    if (!ready.value || loading.value || loadingMore.value || target < 1 || target >= page.value) return Promise.resolve()
    return read(target)
  }
  async function syncInvalidated() {
    if (
      !visible.value ||
      !activeScope ||
      !sameScope(activeScope) ||
      loading.value ||
      loadingMore.value
    )
      return
    const scope = activeScope
    const current = snapshot()
    const ids = new Set(posts.value.map((item) => item.id))
    const dirty = invalidatedPosts(scope.key, scope.school).filter((item) => ids.has(item.id))
    if (dirty.some((item) => item.content)) {
      const changed = new Set(dirty.filter((item) => item.content).map((item) => item.id))
      posts.value = posts.value.filter((item) => !changed.has(item.id))
      const keyword = submittedKeyword.value
      const request = start(keyword)
      const turn = revision
      await request
      if (
        disposed ||
        turn !== revision ||
        !sameScope(scope) ||
        submittedKeyword.value !== keyword ||
        error.value ||
        !page.value
      )
        return
      for (const item of dirty) acknowledgePost(scope.key, scope.school, item.id, item.version)
      return
    }
    if (!dirty.length) return
    const results = await mapConcurrent(dirty, async item => {
      try { return { item, state: await env.api.interactionState(scope.school, item.id), cause: undefined } }
      catch (cause) { return { item, state: undefined, cause } }
    }, current)
    if (!current()) return
    const latest = new Map(invalidatedPosts(scope.key, scope.school).map(item => [item.id, item.version]))
    const changes = new Map<string, PostInteractionState | null>()
    const failures: unknown[] = []
    for (const { item, state, cause } of results) {
      if (latest.get(item.id) !== item.version) continue
      if (state) changes.set(item.id, state)
      else if (cause && typeof cause === 'object' && 'status' in cause && cause.status === 404) changes.set(item.id, null)
      else { failures.push(cause); continue }
      acknowledgePost(scope.key, scope.school, item.id, item.version)
    }
    if (changes.size) posts.value = posts.value.flatMap(post => {
      const change = changes.get(post.id)
      return change === null ? [] : [change ? { ...post, ...change } : post]
    })
    if (failures.length) {
      const message = env.failure(failures[0])
      if (current()) error.value = message
    }
  }
  function sync() {
    if (syncing) return syncing
    const turn = revision
    const request = syncInvalidated().finally(() => {
      if (syncing === request) {
        syncing = undefined
        if (turn !== revision && visible.value && ready.value && !disposed) void sync()
      }
    })
    syncing = request
    return request
  }
  async function show() {
    if (disposed) return
    visible.value = true
    ready.value = false
    const turn = ++showTurn
    try {
      if (!(await env.ensure()) || disposed || !visible.value || turn !== showTurn) return
      if (activeScope && !sameScope(activeScope)) scopeChanged()
      if (knownUser !== env.scope()?.user) scopeChanged()
      ready.value = true
      await sync()
    } catch (cause) {
      if (disposed || turn !== showTurn) return
      const message = env.failure(cause)
      if (!disposed && turn === showTurn) error.value = message
    }
  }
  return {
    draftKeyword,
    submittedKeyword,
    posts,
    page,
    hasMore,
    loading,
    loadingMore,
    error,
    moreError,
    inputError,
    visible,
    ready,
    submit,
    refresh,
    loadMore,
    loadPage,
    sync,
    show,
    scopeChanged,
    clear: () => reset(true),
    hide: () => {
      visible.value = false
      showTurn++
    },
    dispose: () => {
      reset(true)
      disposed = true
      visible.value = false
      showTurn++
    },
  }
}
