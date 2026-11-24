import { afterEach, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick, ref } from 'vue'
import type { HomeMenuTag } from '../src/features/community/api'
import * as tagState from '../src/features/community/publish-tags'
import * as draftState from '../src/features/community/post-draft'
import * as exitState from '../src/features/community/publish-exit'
import * as draftLimits from '../src/features/community/draft-limits'
import { component, findAll, mount, tap, text } from './support/vue-component'

afterEach(() => vi.unstubAllGlobals())
const blank = defineComponent({ setup: () => () => h('view') })
const phone = defineComponent({ setup: (_, { expose }) => { expose({ ensure: () => Promise.resolve(true), close: vi.fn() }); return () => h('view') } })
const picker = component(new URL('../src/features/community/PublishTagPicker.vue', import.meta.url), { './publish-tags': tagState })
const candidates: HomeMenuTag[] = [{ id: 'campus', name: '校园', menu_status: 'hot', source_type: 'admin', enable_status: 'enabled', sort_order: 0, revision: 1, created_at: '' }]
async function settle() {
  for (let index = 0; index < 12; index++) await Promise.resolve()
  await nextTick()
}
function setup(options: { draft?: Record<string, unknown>; edit?: boolean } = {}) {
  let scope = 'scope'
  const listeners: (() => void)[] = []
  const communityApi = {
    post: vi.fn().mockResolvedValue({ author_id: 'account', title: '', content: '原帖', tags: ['校园', '旧标签'], images: [], revision: 3 }),
    create: vi.fn().mockResolvedValue({ id: 'post', revision: 1, audit_status: 'pending' }),
    update: vi.fn().mockResolvedValue({ id: 'post', revision: 4, audit_status: 'pending' }),
  }
  const menu = { public: vi.fn().mockResolvedValue({ tags: candidates }) }
  const storage = new Map<string, unknown>()
  if (options.draft) storage.set(draftState.postDraftKey('account', 'school', options.edit ? 'post' : undefined), options.draft)
  const keepDraftImages = vi.fn().mockResolvedValue({ lease_days: 7 })
  const leave = vi.fn().mockResolvedValue(undefined)
  const onScopeChange = (callback: () => void) => { listeners.push(callback); return vi.fn() }
  vi.stubGlobal('uni', {
    getStorageSync: (key: string) => storage.get(key),
    setStorageSync: (key: string, value: unknown) => storage.set(key, JSON.parse(JSON.stringify(value))),
    removeStorageSync: (key: string) => storage.delete(key),
    hideKeyboard: vi.fn(), showToast: vi.fn(), showModal: vi.fn(), navigateBack: leave,
  })
  const screen = component(new URL('../src/features/community/Publish.vue', import.meta.url), {
    '../../shared/AppIcon.vue': { default: blank },
    './post-invalidations': { invalidatePost: vi.fn() },
    './post-draft': draftState,
    './draft-limits': draftLimits,
    './api': { communityApi, homeMenuApi: menu, normalizePostContent: (value: unknown) => value },
    '../school-context/context': { schoolContext: { id: 'school', schools: [{ id: 'school', name: '学校' }] }, requireSchool: vi.fn().mockResolvedValue(true), scopeKey: () => scope, onScopeChange },
    '../identity/session': { session: { user: { id: 'account' } } },
    '../media/files': { keepDraftImages },
    '../operations/policy': { uuid: () => 'draft-id' },
    '../media/ImageUploads.vue': { default: blank },
    '../../shared/use-page': {
      usePage: (clear: () => void) => {
        onScopeChange(clear)
        return { busy: ref(false), error: ref(''), run: async (action: (current: () => boolean) => Promise<void>) => action(() => true) }
      },
      confirm: vi.fn().mockResolvedValue(true), handlePageError: (error: Error) => error.message,
    },
    '../../shared/PageStatus.vue': { default: blank },
    '../miniapp-users/PhoneBindingPanel.vue': { default: phone },
    './publish-exit': exitState,
    '../../app/navigation': { leavePublish: leave, finishPublish: leave },
    '../identity/return-target': { clearReturnTarget: vi.fn() },
    './PublishTagPicker.vue': { default: picker },
    './publish-tags': tagState,
  })
  const mounted = mount(screen, options.edit ? { id: 'post' } : {})
  const button = (label: string) => findAll(mounted.root, target => target.type === 'button' && text(target) === label)[0]
  const openTags = () => tap(findAll(mounted.root, target => target.props['aria-label'] === '选择标签')[0])
  const write = (value: string) => (findAll(mounted.root, target => target.type === 'textarea')[0].props.onInput as (event: unknown) => void)({ detail: { value } })
  return { mounted, button, openTags, write, communityApi, menu, storage, leave, changeScope: () => { scope = 'next'; listeners.forEach(callback => callback()) } }
}

it('keeps candidates off the form and submits only confirmed names with IDs', async () => {
  const { mounted, openTags, button, write, communityApi } = setup()
  await mounted.exposed.value.show(); await settle()
  expect(findAll(mounted.root, target => target.type === 'button' && text(target).includes('校园'))).toHaveLength(0)
  expect(findAll(mounted.root, target => target.type === 'input')).toHaveLength(0)
  write('正文'); openTags(); await settle()
  tap(findAll(mounted.root, target => target.props['aria-label'] === '校园，未选中')[0]); await settle()
  tap(button('确定')); await settle()
  tap(button('发布')); await settle()
  expect(communityApi.create).toHaveBeenCalledWith('school', { title: '', content: '正文', images: [], tags: ['校园'], candidate_tag_ids: ['campus'] })
  mounted.unmount()
})

it('limits pasted drafts and avoids rewriting an unchanged draft on repeated hides', async () => {
  const { mounted, write, storage } = setup()
  await mounted.exposed.value.show(); await settle()
  const save = vi.spyOn(uni, 'setStorageSync')
  write('😀'.repeat(20001))
  await settle()
  mounted.exposed.value.hide()
  expect(storage.get(draftState.postDraftKey('account', 'school'))).toMatchObject({ content: '😀'.repeat(20000) })
  mounted.exposed.value.hide()
  expect(save).toHaveBeenCalledOnce()
  await mounted.exposed.value.show()
  write('后续修改')
  mounted.exposed.value.hide()
  expect(save).toHaveBeenCalledTimes(2)
  mounted.unmount()
})

it('does not overwrite an oversized legacy draft when auto-save is refused', async () => {
  const original = { content: 'x'.repeat(20001) }
  const { mounted, storage } = setup({ draft: original })
  await mounted.exposed.value.show(); await settle()
  mounted.exposed.value.hide()
  expect(storage.get(draftState.postDraftKey('account', 'school'))).toEqual(original)
  expect(uni.showToast).toHaveBeenCalledWith(expect.objectContaining({ icon: 'none' }))
  mounted.unmount()
})

it('preserves pending legacy draft names across hides and blocks submission until confirmed removal', async () => {
  const { mounted, openTags, button, communityApi, storage } = setup({ draft: { content: '草稿正文', tags: '校园，自定义' } })
  await mounted.exposed.value.show(); await settle()
  tap(button('发布')); await settle()
  expect(communityApi.create).not.toHaveBeenCalled()
  mounted.exposed.value.hide(); await settle()
  const saved = storage.get(draftState.postDraftKey('account', 'school')) as Record<string, unknown>
  expect(saved.schema_version).toBe(3)
  expect(saved.tag_selection).toMatchObject({ unresolved: ['自定义'] })
  await mounted.exposed.value.show(); openTags(); await settle()
  tap(button('自定义 · 确认移除')); await settle()
  tap(button('确定')); await settle()
  tap(button('发布')); await settle()
  expect(communityApi.create).toHaveBeenCalledOnce()
  expect(storage.size).toBe(0)
  mounted.unmount()
})

it('restores IDs for editable candidates and retains original historical names', async () => {
  const { mounted, button, communityApi } = setup({ edit: true })
  await mounted.exposed.value.show(); await settle()
  tap(button('发布')); await settle()
  expect(communityApi.update).toHaveBeenCalledWith('school', 'post', { title: '', content: '原帖', images: [], tags: ['校园', '旧标签'], candidate_tag_ids: ['campus'], revision: 3 })
  mounted.unmount()
})

it('closes a temporary selection on cancel and scope change without leaving or saving it', async () => {
  const { mounted, openTags, leave, changeScope, storage } = setup()
  await mounted.exposed.value.show(); openTags(); await settle()
  tap(findAll(mounted.root, target => target.props['aria-label'] === '校园，未选中')[0]); await settle()
  mounted.exposed.value.cancel(); await settle()
  expect(leave).not.toHaveBeenCalled()
  expect(text(mounted.root)).not.toContain('已选 1')
  openTags(); await settle()
  changeScope(); await settle()
  mounted.exposed.value.hide()
  expect(findAll(mounted.root, target => target.props.role === 'dialog')).toHaveLength(0)
  expect(storage.size).toBe(0)
  mounted.unmount()
})

it('refreshes stale candidates without retrying or discarding the draft', async () => {
  const { mounted, button, menu, communityApi, storage } = setup({ draft: { content: '保留正文', tags: '校园' } })
  await mounted.exposed.value.show(); await settle()
  communityApi.create.mockRejectedValueOnce(Object.assign(new Error('候选标签已失效，请重新选择'), { code: 'validation' }))
  menu.public.mockResolvedValueOnce({ tags: [] })
  tap(button('发布')); await settle()
  expect(communityApi.create).toHaveBeenCalledOnce()
  expect(text(mounted.root)).toContain('候选已刷新')
  expect((findAll(mounted.root, target => target.type === 'textarea')[0].props.value)).toBe('保留正文')
  mounted.exposed.value.hide()
  expect(storage.get(draftState.postDraftKey('account', 'school'))).toMatchObject({ tag_selection: { unresolved: ['校园'] } })
  mounted.unmount()
})

it.each([0, 300])('keeps %s candidates out of the main form and permits untagged publication', async count => {
  const { mounted, button, write, menu, communityApi } = setup()
  menu.public.mockResolvedValueOnce({ tags: Array.from({ length: count }, (_, index) => ({ ...candidates[0], id: `tag-${index}`, name: `候选${index}` })) })
  await mounted.exposed.value.show(); await settle()
  expect(findAll(mounted.root, target => target.type === 'button' && text(target).includes('候选'))).toHaveLength(0)
  write('无需标签的正文'); tap(button('发布')); await settle()
  expect(communityApi.create).toHaveBeenCalledWith('school', expect.objectContaining({ tags: [], candidate_tag_ids: [] }))
  mounted.unmount()
})

it('requires reconfirming refreshed historical selections before an edit can be retried', async () => {
  const { mounted, button, openTags, menu, communityApi } = setup({ edit: true })
  await mounted.exposed.value.show(); await settle()
  communityApi.update.mockRejectedValueOnce(Object.assign(new Error('候选标签已失效，请重新选择'), { code: 'validation' }))
  menu.public.mockResolvedValueOnce({ tags: [] })
  tap(button('发布')); await settle()
  tap(button('发布')); await settle()
  expect(communityApi.update).toHaveBeenCalledOnce()
  openTags(); await settle(); tap(button('确定')); await settle()
  tap(button('发布')); await settle()
  expect(communityApi.update).toHaveBeenCalledTimes(2)
  expect(communityApi.update).toHaveBeenLastCalledWith('school', 'post', expect.objectContaining({ tags: ['校园', '旧标签'], candidate_tag_ids: [] }))
  mounted.unmount()
})
