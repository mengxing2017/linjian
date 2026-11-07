import {expect,it} from 'vitest'
import {navigation,visibleNavigationKeys} from './navigation'
it('exposes complete forum management without sending capabilities',()=>{
 expect(navigation.map(v=>v.key)).toEqual(expect.arrayContaining(['posts','comments','cases','resources','platform-resources','members','settings','schools','accounts']))
 expect(navigation.every(v=>v.domain==='forum')).toBe(true)
 expect(navigation.map(v=>v.key)).not.toEqual(expect.arrayContaining(['mobile-execution-tasks','broadcast-tasks']))
})
it('keeps platform account and school management separate from school operators',()=>{
 expect(visibleNavigationKeys(false,'operator')).toContain('posts')
 expect(visibleNavigationKeys(false,'operator')).not.toContain('accounts')
 expect(visibleNavigationKeys(false,'admin')).toContain('members')
 expect(visibleNavigationKeys(false,'admin')).not.toContain('schools')
 expect(visibleNavigationKeys(false,'member')).toEqual([])
 expect(visibleNavigationKeys(true)).toContain('accounts')
})
