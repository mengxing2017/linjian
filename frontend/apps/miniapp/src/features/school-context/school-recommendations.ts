import { ref, shallowRef } from 'vue'
import type { SchoolRecommendation, Cancellation } from './api'
import { LocationFailure, type Coordinates } from './location'

type State = 'idle' | 'authorizing' | 'privacy' | 'privacy-denied' | 'locating' | 'loading' | 'success' | 'empty' | 'denied' | 'failed'
export function createSchoolRecommendations(dependencies: {
  privacyRequired: () => Promise<boolean>
  authorize: () => Promise<void>
  locate: () => Promise<Coordinates>
  request: (coordinates: Coordinates, signal: Cancellation) => Promise<SchoolRecommendation[]>
  failure: (error: unknown) => void
}) {
  const state = ref<State>('idle'), error = ref(''), items = shallowRef<SchoolRecommendation[]>([])
  let generation = 0
  const timers = new Map<ReturnType<typeof setTimeout>, () => void>()
  let cancel = () => {}
  function timed<Value>(operation: Promise<Value>, milliseconds: number): Promise<Value> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { timers.delete(timer); reject(new Error('请求超时，请重试或手动选择学校')) }, milliseconds)
      timers.set(timer, () => reject(new Error('推荐已取消')))
      operation.then(resolve, reject).finally(() => { clearTimeout(timer); timers.delete(timer) })
    })
  }
  function clear() {
    generation++
    cancel()
    timers.forEach((reject, timer) => { clearTimeout(timer); reject() }); timers.clear()
    state.value = 'idle'; error.value = ''; items.value = []
  }
  function decline() { clear(); state.value = 'privacy-denied'; error.value = '未同意位置使用，本次不提供推荐；仍可手动搜索选择' }
  async function recommend() {
    if (['authorizing', 'locating', 'loading'].includes(state.value)) return
    cancel()
    const listeners = new Set<() => void>()
    const signal: Cancellation & { aborted: boolean } = {
      aborted: false,
      addEventListener: (_type, callback) => { listeners.add(callback) },
      removeEventListener: (_type, callback) => { listeners.delete(callback) },
    }
    const abort = () => { signal.aborted = true; listeners.forEach(callback => callback()); listeners.clear() }
    cancel = abort
    const turn = ++generation
    const current = () => turn === generation
    items.value = []; error.value = ''; state.value = 'authorizing'
    try {
      const privacy = await timed(dependencies.privacyRequired(), 5000)
      if (!current()) return
      if (privacy) { state.value = 'privacy'; return }
      await dependencies.authorize()
      if (!current()) return
      state.value = 'locating'
      const coordinates = await timed(dependencies.locate(), 10000)
      if (!current()) return
      state.value = 'loading'
      const result = await timed(dependencies.request(coordinates, signal), 5000)
      if (!current()) return
      items.value = result
      state.value = result.length ? 'success' : 'empty'
    } catch (failure) {
      if (!current()) return
      abort()
      state.value = failure instanceof LocationFailure && failure.reason === 'denied' ? 'denied' : 'failed'
      error.value = failure instanceof Error ? failure.message : '附近学校加载失败，请重试或手动选择'
      dependencies.failure(failure)
    }
  }
  return { state, error, items, clear, decline, recommend }
}
