import { afterEach, expect, it, vi } from 'vitest'
import { defineComponent, h } from 'vue'
import { createOnboarding } from '../src/features/identity/use-onboarding'
import type { OnboardingProgress, OnboardingReady } from '../src/features/identity/api'
import { component, findAll, mount, text } from './support/vue-component'

afterEach(() => vi.unstubAllGlobals())

const documents = [{ id: 'privacy_policy' as const, kind: 'privacy_policy' as const, revision: 1, updated_at: '2026-11-24T00:00:00Z' }]
const pending: OnboardingProgress = { id: 'current', state: 'school_required', ticket: 'ticket', expires_at: '2026-11-24T00:10:00Z', first_login: false, school_id: 'school', documents, accepted: [] }
const ready: OnboardingReady = { id: 'current', state: 'ready', token: 'token', user: { id: 'user', username: 'user', display_name: '成员', platform_admin: false, bound_school_id: 'school', grants: [] } }

function setup(initial = pending) {
  const identity = { onboarding: vi.fn().mockResolvedValue(ready), onboardingSchools: vi.fn().mockResolvedValue([]), cancelOnboarding: vi.fn() }
  const form = createOnboarding({ identity, documents: { list: vi.fn().mockResolvedValue(documents) }, start: vi.fn().mockResolvedValue(initial), ready: vi.fn(), requestKey: () => 'request-key' })
  const navigation = { redirectTo: vi.fn(), reLaunch: vi.fn() }
  vi.stubGlobal('uni', navigation)
  const dependencies = { './session': { session: { user: null }, restoreSession: vi.fn() }, './onboarding-form': { form } }
  const login = () => mount(component(new URL('../src/features/identity/Login.vue', import.meta.url), {
    ...dependencies,
    '../../app/credentials': { credentials: { token: '' } },
  }))
  const onboarding = () => mount(component(new URL('../src/features/identity/Onboarding.vue', import.meta.url), {
    ...dependencies,
    '../legal-documents/api': { legalTitles: { privacy_policy: '隐私政策' } },
    '../school-context/api': { schoolsApi: {} },
    '../school-context/SchoolRecommendations.vue': { default: defineComponent({ setup: () => () => h('view') }) },
  }))
  return { form, navigation, login, onboarding }
}

it('keeps returning users on the login route and completes login without school or consent controls', async () => {
  const { form, navigation, login } = setup()
  const mounted = login()
  await form.begin()
  await mounted.exposed.value.show()
  expect(navigation.redirectTo).not.toHaveBeenCalled()
  expect(findAll(mounted.root, target => ['input', 'checkbox', 'checkbox-group'].includes(target.type))).toHaveLength(0)
  mounted.unmount()
})

it('routes only new users to the separate onboarding page', async () => {
  const { form, navigation, login } = setup({ ...pending, first_login: true, school_id: null, state: 'consent_required' })
  const mounted = login()
  await form.begin()
  await mounted.exposed.value.show()
  expect(navigation.redirectTo).toHaveBeenCalledWith({ url: '/pages/user/onboarding' })
  expect(findAll(mounted.root, target => ['input', 'checkbox', 'checkbox-group'].includes(target.type))).toHaveLength(0)
  mounted.unmount()
})

it('hides consent controls for returning users even when school selection is still needed', async () => {
  const { form, onboarding } = setup({ ...pending, school_id: null })
  await form.begin()
  const mounted = onboarding()
  expect(text(mounted.root)).toContain('选择学校')
  expect(text(mounted.root)).not.toContain('协议与政策')
  expect(findAll(mounted.root, target => ['checkbox', 'checkbox-group'].includes(target.type))).toHaveLength(0)
  mounted.unmount()
})

it('shows school selection and consent only on the first onboarding page', async () => {
  const { form, onboarding } = setup({ ...pending, first_login: true, school_id: null, state: 'consent_required' })
  await form.begin()
  const mounted = onboarding()
  expect(text(mounted.root)).toContain('选择学校')
  expect(text(mounted.root)).toContain('协议与政策')
  expect(findAll(mounted.root, target => target.type === 'checkbox')).toHaveLength(1)
  mounted.unmount()
})

it('redirects bound users away from onboarding without showing school selection or consent', async () => {
  const { form, navigation, onboarding } = setup()
  await form.begin()
  const mounted = onboarding()
  await mounted.exposed.value.show()
  expect(navigation.redirectTo).toHaveBeenCalledWith({ url: '/pages/user/login' })
  expect(text(mounted.root)).not.toContain('选择学校')
  expect(text(mounted.root)).not.toContain('协议与政策')
  expect(findAll(mounted.root, target => ['input', 'checkbox', 'checkbox-group'].includes(target.type))).toHaveLength(0)
  mounted.unmount()
})
