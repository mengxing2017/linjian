import { expect, it, vi } from 'vitest'
import { createOnboarding } from '../src/features/identity/use-onboarding'
import type { OnboardingProgress, OnboardingReady, OnboardingResult } from '../src/features/identity/api'
const documents = [{ id: 'user_agreement' as const, kind: 'user_agreement' as const, revision: 1, updated_at: '2026-11-24T00:00:00Z' }, { id: 'privacy_policy' as const, kind: 'privacy_policy' as const, revision: 2, updated_at: '2026-11-24T00:00:00Z' }]
const pending: OnboardingProgress = { id: 'current', state: 'consent_required', ticket: 'ticket', expires_at: '2026-11-24T00:10:00Z', first_login: true, documents, accepted: [], school_id: null }
const ready: OnboardingReady = { id: 'current', state: 'ready', token: 'formal-token', user: { id: 'user', username: 'user', display_name: '成员', platform_admin: false, bound_school_id: 'school', grants: [] } }
function setup(initial: OnboardingProgress = pending) {
  const identity = { onboarding: vi.fn<(action: 'status' | 'confirm' | 'complete', input: unknown) => Promise<OnboardingResult>>(), onboardingSchools: vi.fn().mockResolvedValue([{ id: 'school', name: '学校', short_name: '', province: '', city: '' }]), cancelOnboarding: vi.fn().mockResolvedValue({ state: 'cancelled' as const }) }
  const dependencies = { identity, documents: { list: vi.fn().mockResolvedValue(initial.documents) }, start: vi.fn().mockResolvedValue(initial), ready: vi.fn().mockResolvedValue(undefined), requestKey: vi.fn().mockReturnValue('request-key') }
  const form = createOnboarding(dependencies)
  return { form, dependencies, identity }
}
it('loads documents and schools without authenticating or binding', async () => {
  const { form, dependencies, identity } = setup()
  await form.show()
  form.choose('school')
  expect(form.selected.value).toBe('school')
  expect(form.checked.value).toEqual({})
  expect(dependencies.start).not.toHaveBeenCalled()
  expect(identity.onboarding).not.toHaveBeenCalled()
})
it('obtains only an onboarding ticket for recommendations and still requires a manual school selection', async () => {
  const { form, dependencies, identity } = setup()
  await form.show()
  expect(await form.recommendationTicket()).toBe(pending.ticket)
  expect(form.selected.value).toBe('')
  expect(identity.onboarding).not.toHaveBeenCalled()
  expect(dependencies.ready).not.toHaveBeenCalled()
  expect(form.checked.value).toEqual({})
  form.choose('outside-the-nearest-five')
  expect(form.selected.value).toBe('outside-the-nearest-five')
})
it('does not complete first login with one or both unchecked', async () => {
  const { form, dependencies, identity } = setup()
  await form.show(); form.choose('school'); form.toggle(['user_agreement'])
  await form.submit()
  expect(form.error.value).toContain('勾选全部')
  expect(identity.onboarding).not.toHaveBeenCalled()
  expect(dependencies.ready).not.toHaveBeenCalled()
})
it('submits current independent versions before the actual school binding and session', async () => {
  const { form, dependencies, identity } = setup()
  identity.onboarding.mockResolvedValueOnce({ ...pending, state: 'school_required', accepted: documents.map(({ kind, revision }) => ({ kind, revision })) }).mockResolvedValueOnce(ready)
  await form.show(); form.choose('school'); form.toggle(['user_agreement', 'privacy_policy'])
  await form.submit()
  expect(identity.onboarding.mock.calls[0]).toEqual(['confirm', { ticket: 'ticket', acceptances: [{ kind: 'user_agreement', revision: 1 }, { kind: 'privacy_policy', revision: 2 }] }])
  expect(identity.onboarding.mock.calls[1]).toEqual(['complete', { ticket: 'ticket', school_id: 'school', request_key: 'request-key' }])
  expect(dependencies.ready).toHaveBeenCalledWith(ready)
})
it('does not introduce consent for empty configuration', async () => {
  const empty = { ...pending, documents: [], state: 'school_required' as const }
  const { form, identity, dependencies } = setup(empty)
  identity.onboarding.mockResolvedValueOnce(empty).mockResolvedValueOnce(ready)
  await form.show(); form.choose('school'); await form.submit()
  expect(identity.onboarding.mock.calls[0][1]).toEqual({ ticket: 'ticket', acceptances: [] })
  expect(dependencies.ready).toHaveBeenCalledOnce()
})
it('keeps an unchanged selection on reading return and clears only changed versions', async () => {
  const { form, dependencies } = setup()
  await form.show(); form.choose('school'); form.toggle(['user_agreement', 'privacy_policy'])
  await form.show()
  expect(form.checked.value).toEqual({ user_agreement: 1, privacy_policy: 2 })
  dependencies.documents.list.mockResolvedValue([{ ...documents[0] }, { ...documents[1], revision: 3 }])
  await form.show()
  expect(form.checked.value).toEqual({ user_agreement: 1 })
  expect(form.selected.value).toBe('school')
})
it('allows existing successful users without repeated consent and preserves their school', async () => {
  const existing = { ...pending, first_login: false, state: 'school_required' as const, school_id: 'school' }
  const { form, identity, dependencies } = setup(existing)
  identity.onboarding.mockResolvedValueOnce(ready)
  dependencies.documents.list.mockRejectedValue(new Error('documents unavailable'))
  await form.begin()
  form.choose('wrong-school')
  expect(form.selected.value).toBe('school')
  expect(identity.onboarding.mock.calls).toEqual([['complete', { ticket: 'ticket', school_id: 'school', request_key: 'request-key' }]])
  expect(dependencies.documents.list).not.toHaveBeenCalled()
  expect(identity.onboardingSchools).not.toHaveBeenCalled()
  expect(dependencies.ready).toHaveBeenCalledOnce()
})
it('retries an existing login with the same completion key after a lost response', async () => {
  const existing = { ...pending, first_login: false, state: 'school_required' as const, school_id: 'school' }
  const { form, identity, dependencies } = setup(existing)
  identity.onboarding.mockRejectedValueOnce(new Error('response lost'))
  await form.begin()
  expect(form.error.value).toBe('response lost')
  expect(dependencies.ready).not.toHaveBeenCalled()
  identity.onboarding.mockResolvedValueOnce(existing).mockResolvedValueOnce(ready)
  await form.begin()
  expect(identity.onboarding.mock.calls.map(([action]) => action)).toEqual(['complete', 'status', 'complete'])
  expect(identity.onboarding.mock.calls[2][1]).toEqual(identity.onboarding.mock.calls[0][1])
  expect(dependencies.start).toHaveBeenCalledOnce()
  expect(dependencies.ready).toHaveBeenCalledWith(ready)
})
it('recovers an existing login via status without loading onboarding resources', async () => {
  const existing = { ...pending, first_login: false, state: 'school_required' as const, school_id: 'school' }
  const { form, identity, dependencies } = setup(existing)
  identity.onboarding.mockRejectedValueOnce(new Error('response lost'))
  await form.begin()
  identity.onboarding.mockResolvedValueOnce(ready)
  await form.show()
  expect(dependencies.ready).toHaveBeenCalledWith(ready)
  expect(dependencies.documents.list).not.toHaveBeenCalled()
  expect(identity.onboardingSchools).not.toHaveBeenCalled()
})
it('retains server-confirmed versions after binding fails and recovers ready via status', async () => {
  const { form, identity, dependencies } = setup()
  identity.onboarding.mockResolvedValueOnce({ ...pending, state: 'school_required', accepted: documents }).mockRejectedValueOnce(new Error('response lost'))
  await form.show(); form.choose('school'); form.toggle(['user_agreement', 'privacy_policy']); await form.submit()
  expect(form.error.value).toBe('response lost')
  expect(form.checked.value).toEqual({ user_agreement: 1, privacy_policy: 2 })
  expect(dependencies.ready).not.toHaveBeenCalled()
  identity.onboarding.mockResolvedValueOnce(ready)
  await form.show()
  expect(dependencies.ready).toHaveBeenCalledWith(ready)
})
it('treats configuration failure as an error, not an empty requirement set', async () => {
  const { form, dependencies } = setup()
  dependencies.documents.list.mockRejectedValue(new Error('offline'))
  await form.show(); await form.submit()
  expect(form.loaded.value).toBe(false)
  expect(dependencies.start).not.toHaveBeenCalled()
  expect(dependencies.ready).not.toHaveBeenCalled()
})
it('cancels without accepting or navigating and ignores late identity results', async () => {
  const { form, dependencies } = setup()
  let finish!: (value: OnboardingResult) => void
  dependencies.start.mockReturnValue(new Promise(resolve => { finish = resolve }))
  await form.show(); form.choose('school'); form.toggle(['user_agreement', 'privacy_policy'])
  const submitting = form.submit()
  await form.cancel(); finish(pending); await submitting
  expect(form.progress.value).toBeUndefined()
  expect(form.checked.value).toEqual({})
  expect(dependencies.ready).not.toHaveBeenCalled()
})
it('starts onboarding once and keeps the ticket for the onboarding page', async () => {
  const { form, dependencies } = setup()
  await form.begin()
  expect(dependencies.start).toHaveBeenCalledOnce()
  expect(form.progress.value).toEqual(pending)
  await form.begin()
  expect(dependencies.start).toHaveBeenCalledOnce()
})
it('sends ready onboarding results straight to the ready callback', async () => {
  const { form, dependencies } = setup()
  dependencies.start.mockResolvedValueOnce(ready)
  await form.begin()
  expect(dependencies.ready).toHaveBeenCalledWith(ready)
  expect(form.progress.value).toBeUndefined()
})
