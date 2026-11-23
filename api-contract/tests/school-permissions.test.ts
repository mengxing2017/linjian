import { expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { createClient } from '../src/http/client'
import { createSchoolPermissionsClient } from '../src/modules/schools'

const fixture = (name: string) => JSON.parse(readFileSync(new URL(`../fixtures/${name}.json`, import.meta.url), 'utf8'))
it('provides all 61 leaves with valid dependencies and separate role defaults', () => {
  const catalog = fixture('school-permission-catalog').data.attributes
  const keys = new Set(catalog.permissions.map((item: { key: string }) => item.key))
  expect(keys.size).toBe(61)
  for (const definition of catalog.permissions) {
    expect(definition.default_by_role.admin).toBe(definition.key !== 'operation.report.export')
    for (const dependency of definition.requires) expect(keys.has(dependency)).toBe(true)
  }
  expect(catalog.permissions.find((item: { id: string }) => item.id === 'P43').applicable_roles).toEqual(['admin'])
  const simulation = catalog.permissions.find((item: { key: string }) => item.key === 'content.simulation.manage')
  expect(simulation.applicable_roles).toEqual(['admin', 'operator'])
  expect(simulation.default_by_role).toEqual({ admin: true, operator: true })
  expect(fixture('school-member-permissions').data.attributes.effective_permissions).toContain('content.simulation.manage')
})
it('sends versioned full replacements and preserves revision strings without numeric rounding', async () => {
  const transport = vi.fn().mockResolvedValue({status:200,headers:{'content-type':'application/vnd.api+json'},body:fixture('school-member-permissions')})
  const api = createSchoolPermissionsClient(createClient(transport))
  const input = fixture('school-member-authorization-input')
  const result = await api.save('school-id','operator-id',input.data.attributes)
  expect(transport.mock.calls[0][0].path).toBe('/api/v1/schools/school-id/members/operator-id/authorization')
  expect(JSON.parse(transport.mock.calls[0][0].body)).toEqual(input)
  expect(result.revision).toBe('9007199254740994')
})
it('separates management members from global candidates and sends server filters', async () => {
  const transport = vi.fn().mockResolvedValue({status:200,headers:{'content-type':'application/vnd.api+json'},body:{data:[],meta:{page:{number:2,size:30,has_more:false}}}})
  const api = createSchoolPermissionsClient(createClient(transport))
  await api.members('school',{page:2,search:'运营 & 管理',role:'operator',has_overrides:true})
  await api.candidates('school',{page:2})
  const members = new URL(transport.mock.calls[0][0].path,'https://local.example')
  expect(members.searchParams.get('search')).toBe('运营 & 管理')
  expect(members.searchParams.get('page[number]')).toBe('2')
  expect(members.searchParams.get('has_overrides')).toBe('true')
  expect(transport.mock.calls[1][0].path).toContain('/member-candidates?')
})
