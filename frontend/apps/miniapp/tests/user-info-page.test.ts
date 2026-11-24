import { expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick } from 'vue'
import * as userInfo from '../src/features/community/use-user-info'
import { component, findAll, mount, tap, text } from './support/vue-component'

function setup() {
  const profile = vi.fn().mockResolvedValue({
    id: 'user', nickname: '我的昵称', avatar_url: null, avatar_asset_id: null,
    school_name: '我的学校', post_count: 3, created_at: '2026-11-24T00:00:00Z', status: 'normal', revision: 1,
  })
  const closePhone = vi.fn()
  const phone = defineComponent({ setup: (_, { expose }) => { expose({ close: closePhone }); return () => h('view', '手机号管理') } })
  const blank = defineComponent({ setup: () => () => h('view') })
  const status = defineComponent({ props: ['error'], emits: ['retry'], setup: (props, { emit }) => () => h('button', { onTap: () => emit('retry') }, props.error) })
  const screen = component(new URL('../src/features/community/UserInfo.vue', import.meta.url), {
    '../identity/session': { restoreSession: vi.fn(), refreshUser: vi.fn(), session: { user: { id: 'user' } } },
    '../miniapp-users/api': { miniappUserApi: { profile } },
    '../miniapp-users/avatar-files': { uploadAvatar: vi.fn() },
    '../miniapp-users/Avatar.vue': { default: blank },
    '../miniapp-users/PhoneBindingPanel.vue': { default: phone },
    '../../shared/use-page': { confirm: vi.fn().mockResolvedValue(true), handlePageError: (cause: Error) => cause.message },
    '../../shared/PageStatus.vue': { default: status },
    '../school-context/context': { scopeKey: () => 'scope', onScopeChange: () => vi.fn() },
    './use-user-info': userInfo,
  })
  return { mounted: mount(screen), profile, closePhone }
}

it('loads only personal information and preserves profile refresh and phone management', async () => {
  const { mounted, profile, closePhone } = setup()
  await mounted.exposed.value.show()
  await nextTick()
  expect(profile).toHaveBeenCalledOnce()
  expect(text(mounted.root)).toContain('个人信息')
  expect(text(mounted.root)).toContain('保存头像昵称')
  expect(text(mounted.root)).toContain('手机号管理')
  expect(text(mounted.root)).toContain('我的学校')
  for (const title of ['平台服务', '用户协议', '隐私政策']) expect(text(mounted.root)).not.toContain(title)
  await mounted.exposed.value.refresh()
  expect(profile).toHaveBeenCalledTimes(2)
  mounted.exposed.value.hide()
  expect(closePhone).toHaveBeenCalledOnce()
  mounted.unmount()
})

it('retries a failed profile read without school, contacts or policy requests', async () => {
  const { mounted, profile } = setup()
  profile.mockRejectedValueOnce(new Error('个人信息加载失败'))
  await mounted.exposed.value.show()
  await nextTick()
  expect(text(mounted.root)).toContain('个人信息加载失败')
  tap(findAll(mounted.root, target => target.type === 'button')[0])
  await vi.waitFor(() => expect(text(mounted.root)).toContain('保存头像昵称'))
  expect(profile).toHaveBeenCalledTimes(2)
  mounted.unmount()
})
