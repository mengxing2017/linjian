import { shallowRef, ref, nextTick, onUnmounted } from 'vue'
import { restoreSession, session } from '../features/identity/session'
import { credentials } from './credentials'
import { saveReturnTarget, validPostTarget } from '../features/identity/return-target'
import {
  onReady,
  onShow,
  onHide,
  onPullDownRefresh,
  onReachBottom,
  onShareAppMessage,
  onLoad,
} from '@dcloudio/uni-app'
interface Screen {
  show?: () => void | Promise<void>
  hide?: () => void
  refresh?: () => void | Promise<void>
  loadMore?: () => void | Promise<void>
  share?: () => { title: string; path: string; imageUrl?: string }
}
export function useProtectedScreen(share = false) {
  const screen = shallowRef<Screen>()
  const allowed = ref(false)
  let generation = 0, visible = true, dispatched = false
  let entering: Promise<void> | undefined
  const stop = credentials.subscribe(() => {
    generation++
    allowed.value = false
    entering = undefined
    dispatched = false
    if (visible) void enter()
  })
  onUnmounted(() => { generation++; stop() })
  onLoad(query => {
    if (share && validPostTarget(String(query?.school ?? ''), String(query?.id ?? ''))) saveReturnTarget(String(query?.school), String(query?.id))
  })
  function enter() {
    if (entering) return entering
    const turn = ++generation
    const operation = (async () => {
      try {
        await restoreSession()
        if (turn !== generation || !visible) return
        if (!session.user?.bound_school_id) {
          allowed.value = false
          await uni.reLaunch({ url: '/pages/user/login' })
          return
        }
        allowed.value = true
        await nextTick()
        if (turn === generation && visible && screen.value) { dispatched = true; await screen.value.show?.() }
      } catch {
        allowed.value = false
        if (turn === generation && visible) await uni.reLaunch({ url: '/pages/user/login' })
      }
    })()
    entering = operation
    void operation.finally(() => { if (entering === operation) entering = undefined })
    return entering
  }
  onReady(() => { if (!dispatched) void enter() })
  onShow(() => { visible = true; void enter() })
  onHide(() => { visible = false; generation++; entering = undefined; dispatched = false; screen.value?.hide?.() })
  onPullDownRefresh(() => { Promise.resolve(allowed.value ? screen.value?.refresh?.() : enter()).finally(() => uni.stopPullDownRefresh()) })
  onReachBottom(() => { if (allowed.value) void screen.value?.loadMore?.() })
  if (share) onShareAppMessage(() => allowed.value ? screen.value?.share?.() ?? { title: '校园社区', path: '/pages/user/login' } : { title: '校园社区', path: '/pages/user/login' })
  return { screen, allowed }
}
/** Uni page hooks belong to registered page wrappers, not child components. */
export function useScreen(share = false) {
  const screen = shallowRef<Screen>()
  let pendingShow = false
  let dispatched = false
  let visible = true
  onReady(() => {
    if (visible && (pendingShow || !dispatched)) {
      pendingShow = false
      dispatched = true
      void screen.value?.show?.()
    }
  })
  onShow(() => {
    visible = true
    if (!screen.value) { pendingShow = true; return }
    pendingShow = false
    dispatched = true
    void screen.value?.show?.()
  })
  onHide(() => { visible = false; pendingShow = false; screen.value?.hide?.() })
  onPullDownRefresh(() => {
    Promise.resolve(screen.value?.refresh?.()).finally(() => uni.stopPullDownRefresh())
  })
  onReachBottom(() => {
    void screen.value?.loadMore?.()
  })
  if (share)
    onShareAppMessage(
      () => screen.value?.share?.() ?? { title: 'Linjian', path: '/pages/index/index' },
    )
  return screen
}
