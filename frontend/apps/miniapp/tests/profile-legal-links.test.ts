import { afterEach, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick, ref } from 'vue'
import { component, findAll, mount, tap, text } from './support/vue-component'

afterEach(() => vi.unstubAllGlobals())

const summaries = [
  { id: 'user_agreement', kind: 'user_agreement', revision: 2, updated_at: '2026-11-24T00:00:00Z' },
  { id: 'privacy_policy', kind: 'privacy_policy', revision: 3, updated_at: '2026-11-24T00:00:00Z' },
]

function setup() {
  const list = vi.fn().mockResolvedValue(summaries)
  const navigateTo = vi.fn()
  vi.stubGlobal('uni', { navigateTo })
  const blank = defineComponent({ setup: () => () => h('view') })
  const menu = defineComponent({ props: ['title'], emits: ['activate'], setup: (props, { emit }) => () => h('button', { onTap: () => emit('activate') }, props.title) })
  const links = component(new URL('../src/features/legal-documents/LegalLinks.vue', import.meta.url), {
    '../../shared/MenuItem.vue': { default: menu },
    '../../shared/icons': { documentIcon: () => 'application' },
    './api': { legalDocuments: { list }, legalTitles: { user_agreement: '用户协议', privacy_policy: '隐私政策' } },
  })
  const page = { busy: ref(false), error: ref('学校资料加载失败'), show: vi.fn().mockResolvedValue(undefined), hide: vi.fn(), dispose: vi.fn(), resetProfile: vi.fn(), reloadProfile: vi.fn() }
  const screen = component(new URL('../src/features/community/Profile.vue', import.meta.url), {
    '../operations/CustomerContacts.vue': { default: blank },
    '../legal-documents/LegalLinks.vue': { default: links },
    '../../shared/MenuItem.vue': { default: menu },
    '../miniapp-users/Avatar.vue': { default: blank },
    '../miniapp-users/api': { miniappUserApi: {} },
    '../identity/session': { session: { user: { id: 'user', bound_school_id: 'school' } }, restoreSession: vi.fn(), logout: vi.fn() },
    '../school-context/context': { schoolContext: { id: 'school', school: { name: '学校' } }, loadSchool: vi.fn(), scopeKey: () => 'scope', onScopeChange: () => vi.fn() },
    '../../app/credentials': { credentials: { revision: 1 } },
    '../../shared/use-page': { usePage: () => ({ busy: ref(false), error: ref(''), run: vi.fn() }), navigate: vi.fn(), confirm: vi.fn(), handlePageError: vi.fn() },
    '../../shared/PageStatus.vue': { default: blank },
    './use-contacts-page': { createContactsPage: () => page },
  })
  return { mounted: mount(screen), list, navigateTo, page }
}

it('shows both configured policies directly on the profile independently of school loading', async () => {
  const { mounted, list, navigateTo, page } = setup()
  await mounted.exposed.value.show()
  await nextTick()
  expect(list).toHaveBeenCalledOnce()
  expect(page.show).toHaveBeenCalledOnce()
  for (const [title, kind] of [['用户协议', 'user_agreement'], ['隐私政策', 'privacy_policy']]) {
    const entry = findAll(mounted.root, target => target.type === 'button' && text(target) === title)
    expect(entry).toHaveLength(1)
    tap(entry[0])
    expect(navigateTo).toHaveBeenLastCalledWith({ url: `/pages/user/policy?kind=${kind}` })
  }
  mounted.unmount()
})

it('shows only configured policies and refreshes them on re-entry', async () => {
  const { mounted, list } = setup()
  list.mockResolvedValueOnce([summaries[0]])
  await mounted.exposed.value.show()
  await nextTick()
  expect(text(mounted.root)).toContain('用户协议')
  expect(text(mounted.root)).not.toContain('隐私政策')
  mounted.exposed.value.hide()
  await mounted.exposed.value.show()
  await nextTick()
  expect(text(mounted.root)).toContain('隐私政策')
  expect(list).toHaveBeenCalledTimes(2)
  mounted.unmount()
})

it('does not invent entries when no policies have been configured', async () => {
  const { mounted, list } = setup()
  list.mockResolvedValueOnce([])
  await mounted.exposed.value.show()
  await nextTick()
  expect(text(mounted.root)).not.toContain('用户协议')
  expect(text(mounted.root)).not.toContain('隐私政策')
  mounted.unmount()
})

it('offers an independent retry when policies fail to load', async () => {
  const { mounted, list, page } = setup()
  list.mockRejectedValueOnce(new Error('网络异常'))
  await mounted.exposed.value.show()
  await nextTick()
  expect(text(mounted.root)).toContain('协议与政策加载失败')
  tap(findAll(mounted.root, target => target.type === 'button' && text(target) === '重试')[0])
  await nextTick()
  await nextTick()
  expect(text(mounted.root)).toContain('用户协议')
  expect(text(mounted.root)).toContain('隐私政策')
  expect(text(mounted.root)).not.toContain('协议与政策加载失败')
  expect(page.show).toHaveBeenCalledOnce()
  mounted.unmount()
})

it('ignores a late policy response after leaving the profile', async () => {
  const { mounted, list, page } = setup()
  let resolve!: (value: typeof summaries) => void
  list.mockReturnValueOnce(new Promise(finish => { resolve = finish }))
  await mounted.exposed.value.show()
  mounted.exposed.value.hide()
  resolve(summaries)
  await nextTick()
  await nextTick()
  expect(page.hide).toHaveBeenCalledOnce()
  expect(text(mounted.root)).not.toContain('用户协议')
  expect(text(mounted.root)).not.toContain('隐私政策')
  mounted.unmount()
})
