import {afterEach,describe,it,expect,vi} from 'vitest'
import {sourceAccess} from '../src/modules/source-access'
afterEach(()=>vi.unstubAllGlobals())
import {createHash,createHmac} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {resourceType,writeDocument} from '../src/http/protocol'
const fixture=JSON.parse(readFileSync(new URL('../fixtures/source-v2.json',import.meta.url),'utf8'))
describe('published source-v2 bytes',()=>{
 it('writes an empty verification action with no browser credential fields',()=>{
  const doc=JSON.parse(readFileSync(new URL('../fixtures/source-connection-input.json',import.meta.url),'utf8'))
  expect(writeDocument('/api/v2/source-connections',doc.data.attributes)).toEqual(doc)
  expect(doc.data.attributes).not.toHaveProperty('source_id')
 })
 it('matches independently generated UTF-8, empty GET and check vectors',()=>{
  for(const v of fixture.vectors){
   const digest=createHash('sha256').update(v.body,'utf8').digest('hex')
   expect(digest).toBe(v.digest)
   const text=['2',v.key_id,v.method,v.path,v.timestamp,v.nonce,v.identity,digest].join('\n')
   expect(text).toBe(v.canonical)
   expect(createHmac('sha256',Buffer.from(v.secret,'base64url')).update(text,'utf8').digest('hex')).toBe(v.signature)
   expect(createHmac('sha256',Buffer.from(v.secret,'base64url')).update(text+'\n').digest('hex')).not.toBe(v.signature)
  }
 })
 it('selects management action types without changing attributes',()=>{
  expect(resourceType('/api/v2/source-connections')).toBe('source-connections'); expect(writeDocument('/api/v2/source-connections',{revision:1}).data.attributes).toEqual({revision:1})
expect(resourceType('/api/v2/schools/abc/source-bindings')).toBe('source-bindings'); expect(writeDocument('/api/v2/schools/abc/source-bindings',{revision:1}).data.attributes).toEqual({revision:1})
 })
})

it('reads public deployment settings and posts an empty verification action',async()=>{
 const settings=JSON.parse(readFileSync(new URL('../fixtures/source-connection-settings.json',import.meta.url),'utf8'))
 const fetchMock=vi.fn().mockResolvedValueOnce(Response.json(settings,{headers:{'content-type':'application/vnd.api+json'}})).mockResolvedValueOnce(Response.json({data:{type:'source-connections',id:'connection-1',attributes:{revision:1}}},{headers:{'content-type':'application/vnd.api+json'}}))
 vi.stubGlobal('fetch',fetchMock)
 expect(await sourceAccess.connections()).toEqual({id:'list',...settings.data.attributes})
 await sourceAccess.verify()
 const [url,options]=fetchMock.mock.calls[1]
 expect(url).toBe('/api/v2/source-connections')
 expect(options.method).toBe('POST')
 expect(JSON.parse(options.body)).toEqual({data:{type:'source-connections',attributes:{}}})
 expect(JSON.stringify(settings)).not.toContain('secret')
})

it('reads one school route and saves it without a connection choice',async()=>{
 const fixture=JSON.parse(readFileSync(new URL('../fixtures/school-source-binding.json',import.meta.url),'utf8'))
 const fetchMock=vi.fn().mockResolvedValueOnce(Response.json(fixture,{headers:{'content-type':'application/vnd.api+json'}})).mockResolvedValueOnce(Response.json({data:{type:'source-bindings',id:'binding-1',attributes:{}}},{headers:{'content-type':'application/vnd.api+json'}}))
 vi.stubGlobal('fetch',fetchMock)
 const result=await sourceAccess.bindings(fixture.data.id)
 expect(result.binding).toEqual(fixture.data.attributes.binding)
 expect(result).not.toHaveProperty('items')
 await sourceAccess.bind(fixture.data.id,{route_id:'replacement-route',active:true})
 expect(JSON.parse(fetchMock.mock.calls[1][1].body).data.attributes).toEqual({route_id:'replacement-route',active:true})
})

it('writes the Rust school-route input fixture without connection_id',()=>{
 const fixture=JSON.parse(readFileSync(new URL('../fixtures/school-source-binding-input.json',import.meta.url),'utf8'))
 expect(writeDocument('/api/v2/schools/school-1/source-bindings',fixture.data.attributes)).toEqual(fixture)
 expect(fixture.data.attributes).not.toHaveProperty('connection_id')
})
