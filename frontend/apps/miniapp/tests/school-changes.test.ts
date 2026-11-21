import { expect, it } from 'vitest'
import { createSubmissionKeys, newSubmissionKey } from '../src/features/school-changes/submission'

it('keeps retries on the same submission and replaces the key after an edit', () => {
  const key = createSubmissionKeys()
  const payload = { to_school_id: 'target', document_ids: ['document'], reason: '升学' }
  const initial = key(payload)
  expect(key({ ...payload })).toBe(initial)
  expect(key({ ...payload, reason: '选错学校' })).not.toBe(initial)
})
it('scopes keys to a form instance and produces valid UUIDs without storing personal information', () => {
  const payload = { reason: '不应持久化' }
  expect(createSubmissionKeys()(payload)).not.toBe(createSubmissionKeys()(payload))
  expect(newSubmissionKey()).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/)
})
it('creates a new submission key when the declared student relationship changes', () => {
  const key = createSubmissionKeys()
  const payload = { to_school_id: 'target', student_relationship: 'enrolled', document_ids: ['document'] }
  expect(key({ ...payload, student_relationship: 'new_student' })).not.toBe(key(payload))
})
