import { expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { createClient } from '../src/http/client'
import { createHomeMenuClient } from '../src/modules/home-menu'

it('decodes the menu and writes the catalog revision with the complete order', async () => {
  const fixture = JSON.parse(readFileSync(new URL('../fixtures/home-menu.json', import.meta.url), 'utf8'))
  const transport = vi.fn().mockResolvedValueOnce({status:200,headers:{'content-type':'application/vnd.api+json'},body:fixture})
    .mockResolvedValueOnce({status:200,headers:{'content-type':'application/vnd.api+json'},body:{data:null}})
  const api = createHomeMenuClient(createClient(transport))
  const menu = await api.public('school-1')
  expect(menu.tags[0]).toMatchObject({name:'校园',menu_status:'hot'})
  await api.order('school-1', ['tag-1'], menu.settings.catalog_revision)
  expect(JSON.parse(transport.mock.calls[1][0].body)).toEqual({data:{type:'home-menu-order',attributes:{ids:['tag-1'],catalog_revision:2}}})
})
