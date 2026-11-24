import { afterEach, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick } from 'vue'
import * as postDetail from '../src/features/community/use-post-detail'
import { component, findAll, mount, tap, text } from './support/vue-component'

afterEach(() => vi.unstubAllGlobals())

async function setup() {
  const api = {
    post: vi.fn().mockResolvedValue({ id: 'post', author_id: 'author', title: '帖子', content: '正文', images: [], tags: [], visibility: 'visible', audit_status: 'approved', comment_count: 0, like_count: 0 }),
    interactionState: vi.fn().mockResolvedValue({ id: 'post', can_interact: true, can_share: true, visibility: 'visible', audit_status: 'approved' }),
    comments: vi.fn().mockResolvedValue({ items: [], page: { number: 1, has_more: false } }),
    report: vi.fn().mockResolvedValue(undefined),
  }
  const uni = { showActionSheet: vi.fn().mockResolvedValue({ tapIndex: 0 }), pageScrollTo: vi.fn(), showToast: vi.fn() }
  vi.stubGlobal('uni', uni)
  vi.stubGlobal('document', { activeElement: null })
  const blank = defineComponent({ setup: () => () => h('view') })
  const phone = defineComponent({ setup: (_, { expose }) => { expose({ close: vi.fn() }); return () => h('view') } })
  const status = defineComponent({ props: ['error'], setup: props => () => h('text', props.error) })
  const screen = component(new URL('../src/features/community/Detail.vue', import.meta.url), {
    '../../shared/PageWindow.vue': { default: blank },
    '../../shared/AppIcon.vue': { default: blank },
    '../statistics/activity': { recordPage: vi.fn() },
    './post-status': { postVisibilityLabel: () => '' },
    './api': { communityApi: api },
    '../school-context/context': { requireSchool: async () => true, scopeKey: () => 'scope', onScopeChange: () => vi.fn() },
    '../identity/session': { restoreSession: vi.fn(), session: { user: { id: 'user', bound_school_id: 'school' } } },
    '../identity/return-target': { saveReturnTarget: vi.fn(), validPostTarget: () => true },
    '../../shared/use-page': { confirm: vi.fn(), navigate: vi.fn(), handlePageError: (cause: Error) => cause.message },
    './use-post-detail': postDetail,
    './sharing': { postShare: vi.fn() },
    './post-invalidations': { invalidatePost: vi.fn(), invalidatedPosts: () => [], clearPostInvalidations: vi.fn() },
    '../media/PrivateImage.vue': { default: blank },
    '../../shared/PageStatus.vue': { default: status },
    '../miniapp-users/PhoneBindingPanel.vue': { default: phone },
  })
  const mounted = mount(screen, { school: 'school', id: 'post' })
  await mounted.exposed.value.show()
  await nextTick()
  const button = (label: string) => findAll(mounted.root, target => target.type === 'button' && (text(target) === label || target.props['aria-label'] === label))[0]
  async function openReport() {
    tap(button('更多操作'))
    await nextTick()
    await nextTick()
  }
  function reason(value: string) {
    const input = findAll(mounted.root, target => target.type === 'input')[0]
    ;(input.props['onUpdate:modelValue'] as (value: string) => void)(value)
  }
  return { mounted, api, uni, button, openReport, reason }
}

it('hides reporting by default and requires choosing it from the post more menu', async () => {
  const { mounted, api, uni, button, openReport } = await setup()
  expect(text(button('更多操作'))).toBe('⋯')
  expect(text(mounted.root)).not.toContain('举报')
  expect(findAll(mounted.root, target => target.type === 'input')).toHaveLength(0)
  await openReport()
  expect(uni.showActionSheet).toHaveBeenCalledWith({ itemList: ['举报'] })
  expect(text(mounted.root)).toContain('提交举报')
  expect(uni.pageScrollTo).toHaveBeenCalledWith({ selector: '#post-report-form', duration: 200 })
  expect(api.report).not.toHaveBeenCalled()
  mounted.unmount()
})

it('requires a reason and an explicit submission before reporting, then closes the form', async () => {
  const { mounted, api, uni, button, openReport, reason } = await setup()
  await openReport()
  tap(button('提交举报'))
  await nextTick()
  expect(text(mounted.root)).toContain('请填写举报原因')
  expect(api.report).not.toHaveBeenCalled()
  reason('垃圾广告')
  tap(button('提交举报'))
  await nextTick()
  await nextTick()
  expect(api.report).toHaveBeenCalledExactlyOnceWith('school', 'post', '垃圾广告')
  expect(uni.showToast).toHaveBeenCalledWith({ title: '已提交举报', icon: 'none' })
  expect(button('提交举报')).toBeUndefined()
  mounted.unmount()
})

it('allows cancelling the form or the more menu without submitting', async () => {
  const { mounted, api, uni, button, openReport, reason } = await setup()
  await openReport()
  reason('不提交的草稿')
  tap(button('取消'))
  await nextTick()
  expect(button('提交举报')).toBeUndefined()
  uni.showActionSheet.mockRejectedValueOnce({ errMsg: 'showActionSheet:fail cancel' })
  await openReport()
  expect(button('提交举报')).toBeUndefined()
  expect(api.report).not.toHaveBeenCalled()
  await openReport()
  tap(button('提交举报'))
  await nextTick()
  expect(api.report).not.toHaveBeenCalled()
  mounted.unmount()
})

it('retains the form and reason after a failed submission so the user can retry', async () => {
  const { mounted, api, button, openReport, reason } = await setup()
  api.report.mockRejectedValueOnce(new Error('举报提交失败'))
  await openReport()
  reason('垃圾广告')
  tap(button('提交举报'))
  await nextTick()
  await nextTick()
  expect(text(mounted.root)).toContain('举报提交失败')
  expect(button('提交举报')).toBeDefined()
  tap(button('提交举报'))
  await nextTick()
  await nextTick()
  expect(api.report).toHaveBeenLastCalledWith('school', 'post', '垃圾广告')
  expect(button('提交举报')).toBeUndefined()
  mounted.unmount()
})

it.each(['hide', 'change-post'])('ignores stale menu selections after %s', async action => {
  const { mounted, api, uni, button } = await setup()
  let resolve!: (choice: { tapIndex: number }) => void
  uni.showActionSheet.mockReturnValueOnce(new Promise(finish => { resolve = finish }))
  tap(button('更多操作'))
  if (action === 'hide') mounted.exposed.value.hide()
  else { mounted.props.id = 'another-post'; await nextTick() }
  resolve({ tapIndex: 0 })
  await nextTick()
  await nextTick()
  expect(button('提交举报')).toBeUndefined()
  expect(api.report).not.toHaveBeenCalled()
  expect(uni.pageScrollTo).not.toHaveBeenCalled()
  mounted.unmount()
})
