import { expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { createClient } from '../src/http/client'
import { createBlockedWordsClient } from '../src/modules/business'

it('decodes the catalog and writes versioned order and imports using their resource types', async () => {
  const fixture = JSON.parse(readFileSync(new URL('../fixtures/blocked-words.json', import.meta.url), 'utf8'))
  const response = (body: unknown) => ({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body })
  const transport = vi.fn().mockResolvedValueOnce(response(fixture)).mockResolvedValueOnce(response({data:null}))
    .mockResolvedValueOnce(response({data:{type:'blocked-word-imports',id:'job-1',attributes:{count:1}}}))
  const api = createBlockedWordsClient(createClient(transport))
  const catalog = await api.list(undefined, false, '', 'all', 1, false)
  expect(catalog.items[0]).toMatchObject({word:'诈骗',active:true})
  await api.order(undefined,['word-1'],catalog.order_revision)
  expect(JSON.parse(transport.mock.calls[1][0].body)).toEqual({data:{type:'blocked-words',attributes:{ids:['word-1'],order_revision:2}}})
  await api.commitImport(undefined,'job-1','digest')
  expect(JSON.parse(transport.mock.calls[2][0].body)).toEqual({data:{type:'blocked-word-imports',attributes:{digest:'digest'}}})
})
