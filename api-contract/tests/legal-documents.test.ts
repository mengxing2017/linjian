import { expect, it, vi } from 'vitest'
import { createClient, type Transport } from '../src/http/client'
import { createLegalDocumentsClient, validLegalContent } from '../src/modules/legal-documents'
import { createIdentityClient } from '../src/modules/identity'
import empty from '../fixtures/legal-documents-empty.json'
import detail from '../fixtures/legal-document.json'
import progress from '../fixtures/onboarding-progress.json'
import ready from '../fixtures/onboarding-ready.json'
const response = (body: unknown) => ({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body })
it('reads public empty configuration and full text without credentials', async () => {
  const transport = vi.fn<Transport>().mockResolvedValueOnce(response(empty)).mockResolvedValueOnce(response(detail))
  const api = createLegalDocumentsClient(createClient(transport))
  expect(await api.list()).toEqual([])
  expect((await api.read('privacy_policy')).content).toContain('\n')
  expect(transport.mock.calls[0][0].headers).not.toHaveProperty('X-CSRF-Token')
})
it('writes explicit versioned save and clear commands', async () => {
  const transport = vi.fn<Transport>().mockResolvedValue(response(detail))
  const client = createClient(transport)
  client.setCsrf('csrf')
  const api = createLegalDocumentsClient(client)
  await api.save('privacy_policy', 2, '新正文')
  await api.clear('privacy_policy', 3)
  expect(JSON.parse(String(transport.mock.calls[0][0].body)).data).toEqual({ type: 'legal-documents', attributes: { revision: 2, content: '新正文' } })
  expect(JSON.parse(String(transport.mock.calls[1][0].body)).data).toEqual({ type: 'legal-document-clears', attributes: { revision: 3 } })
})
it('does not confuse restricted onboarding progress with a formal session', async () => {
  const transport = vi.fn<Transport>().mockResolvedValueOnce(response(progress)).mockResolvedValueOnce(response(ready))
  const api = createIdentityClient(createClient(transport))
  const first = await api.startOnboarding('code')
  expect(first.state).toBe('consent_required')
  expect(first).not.toHaveProperty('token')
  expect((await api.onboarding('complete', { ticket: 'ticket', school_id: 'school', request_key: 'key' })).state).toBe('ready')
  expect(JSON.parse(String(transport.mock.calls[1][0].body)).data.type).toBe('onboarding')
})
it('validates scalar counts and rejects invisible-only documents', () => {
  for (const value of ['', ' \n　', '\u200b\ufeff']) expect(validLegalContent(value)).toBe(false)
  expect(validLegalContent('😀'.repeat(50000))).toBe(true)
  expect(validLegalContent('😀'.repeat(50001))).toBe(false)
})
