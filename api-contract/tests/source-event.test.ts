import {expect,it} from 'vitest'
import {createHash,createHmac} from 'node:crypto'
import fixture from '../fixtures/source-event-v1.json'
it('shares exact JSON:API UTF-8 request bytes and the HMAC vector with Rust',()=>{
 const doc=JSON.parse(fixture.body)
 expect(doc.data.type).toBe('source-events')
 expect(doc.data.id).toBe(fixture.event.event_id)
 expect(doc.data.attributes).toEqual(fixture.event)
 const canonical=['1',fixture.key_id,'POST','/api/v1/source-events',fixture.timestamp,fixture.nonce,fixture.event.event_id,createHash('sha256').update(fixture.body,'utf8').digest('hex')].join('\n')
 expect(canonical).toBe(fixture.canonical)
 expect(createHmac('sha256',fixture.secret).update(canonical,'utf8').digest('hex')).toBe(fixture.signature)
 expect(doc.data.attributes).not.toHaveProperty('confirmed')
 expect(doc.data.attributes).not.toHaveProperty('school_id')
})
