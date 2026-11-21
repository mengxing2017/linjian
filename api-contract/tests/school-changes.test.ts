import { readFileSync } from 'node:fs'
import { expect, it, vi } from 'vitest'
import { createClient } from '../src/http/client'
import { createSchoolChangesClient } from '../src/modules/school-changes'

const fixture = JSON.parse(readFileSync(new URL('../fixtures/school-change-pending.json', import.meta.url), 'utf8'))
function setup(body: unknown = fixture) {
  const transport = vi.fn().mockResolvedValue({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body })
  return { transport, api: createSchoolChangesClient(createClient(transport)) }
}
it('decodes current, nullable empty and always available policy fixtures', async () => {
  const { api } = setup()
  expect(await api.current()).toEqual({ id: fixture.data.id, ...fixture.data.attributes })
  const empty = JSON.parse(readFileSync(new URL('../fixtures/school-change-empty.json', import.meta.url), 'utf8'))
  expect(await setup(empty).api.current()).toBeNull()
  await expect(setup(empty).api.detail(fixture.data.id)).rejects.toMatchObject({ code: 'invalid_document' })
  const policy = JSON.parse(readFileSync(new URL('../fixtures/school-change-policy.json', import.meta.url), 'utf8'))
  expect((await setup(policy).api.policy()).enabled).toBe(true)
  policy.data.attributes.enabled = false
  await expect(setup(policy).api.policy()).rejects.toMatchObject({ code: 'invalid_document' })
})
it('rejects invalid resource types, states, revisions and missing material availability', async () => {
  for (const patch of [{ status: 'draft' }, { status: 'approved' }, { revision: 0 }, { to_school_id: fixture.data.attributes.from_school_id }, { documents: [{ id: fixture.data.attributes.documents[0].id, mime: 'image/png' }] }, { blocked_reasons: 'none' }]) {
    await expect(setup({ data: { ...fixture.data, attributes: { ...fixture.data.attributes, ...patch } } }).api.current()).rejects.toMatchObject({ code: 'invalid_document' })
  }
  await expect(setup({ data: { ...fixture.data, type: 'posts' } }).api.current()).rejects.toMatchObject({ code: 'invalid_document' })
})
it.each(['approved', 'rejected', 'cancelled', 'invalidated'])('decodes the mutually exclusive %s terminal with cleaned materials', async status => {
  const reviewed = status === 'approved' || status === 'rejected'
  const attributes = { ...fixture.data.attributes, status, revision: 2, closed_at: '2026-11-21T01:00:00Z', reviewed_at: reviewed ? '2026-11-21T01:00:00Z' : null,
    reviewed_by: reviewed ? '10000000-0000-4000-8000-000000000002' : null, personal_data_purged: true,
    declared_student_name: null, reason: null, documents: [{ ...fixture.data.attributes.documents[0], available: false }] }
  expect((await setup({ data: { ...fixture.data, attributes } }).api.current())?.status).toBe(status)
})
it('sends versioned JSON:API actions and binary requests without exposing permanent URLs', async () => {
  const { api, transport } = setup()
  const request = (await api.current())!
  await api.cancel(request)
  expect(JSON.parse(transport.mock.lastCall![0].body)).toEqual({ data: { type: 'miniapp-school-change-cancellations', attributes: { revision: 1 } } })
  await api.approve(request, '已核验目标学校')
  expect(JSON.parse(transport.mock.lastCall![0].body).data.type).toBe('miniapp-school-change-approvals')
  await api.reject(request, '材料不清晰')
  expect(JSON.parse(transport.mock.lastCall![0].body).data.type).toBe('miniapp-school-change-rejections')
  transport.mockResolvedValue({ status: 200, headers: {}, body: new ArrayBuffer(2) })
  await api.document(request.documents[0].id, request.id)
  expect(transport.mock.lastCall![0]).toMatchObject({ binary: true, path: `/api/v1/platform/school-change-requests/${request.id}/documents/${request.documents[0].id}` })
})
it('requires paginated collections and propagates forbidden results', async () => {
  await expect(setup({ data: [] }).api.list()).rejects.toMatchObject({ code: 'invalid_document' })
  const { api, transport } = setup()
  transport.mockResolvedValue({ status: 403, headers: {}, body: { errors: [{ code: 'forbidden' }] } })
  await expect(api.current()).rejects.toMatchObject({ status: 403, code: 'forbidden' })
})
it('scopes related review records to the selected account', async () => {
  const { api, transport } = setup({ data: [], meta: { page: { number: 1, size: 30, has_more: false } } })
  expect((await api.list({ account_id: fixture.data.attributes.account_id })).items).toEqual([])
  expect(transport.mock.lastCall![0].path).toContain(`account_id=${fixture.data.attributes.account_id}`)
})
it('decodes automatic timeout rejections without inventing a human reviewer', async () => {
  const attributes = { ...fixture.data.attributes, status: 'rejected', revision: 2, closed_at: fixture.data.attributes.review_due_at,
    result_reason_code: 'review_timeout', result_reason: '超期未审核，系统自动驳回' }
  expect((await setup({ data: { ...fixture.data, attributes } }).api.current())?.reviewed_by).toBeNull()
  for (const patch of [{ status: 'approved' }, { status: 'pending', closed_at: null }, { reviewed_by: fixture.data.attributes.account_id, reviewed_at: attributes.closed_at }, { result_reason_code: 'manual_rejected' }]) {
    await expect(setup({ data: { ...fixture.data, attributes: { ...attributes, ...patch } } }).api.current()).rejects.toMatchObject({ code: 'invalid_document' })
  }
})
it('rejects unsupported student relationships and missing review deadlines in policy', async () => {
  for (const student_relationship of ['graduated', 'suspended', 'other', undefined]) {
    await expect(setup({ data: { ...fixture.data, attributes: { ...fixture.data.attributes, student_relationship } } }).api.current()).rejects.toMatchObject({ code: 'invalid_document' })
  }
  const policy = JSON.parse(readFileSync(new URL('../fixtures/school-change-policy.json', import.meta.url), 'utf8'))
  policy.data.attributes.review_working_days = 0
  await expect(setup(policy).api.policy()).rejects.toMatchObject({ code: 'invalid_document' })
})
