import { afterEach, expect, it, vi } from 'vitest'
import { executeServiceChannel } from '../src/features/operations/service-actions'
afterEach(()=>vi.unstubAllGlobals())
it('invokes mini-program navigation synchronously in the user gesture stack',()=>{
  const navigateToMiniProgram=vi.fn(),done=vi.fn()
  vi.stubGlobal('uni',{navigateToMiniProgram})
  executeServiceChannel({key:'mini',label:'访问服务',action:{kind:'mini_program',app_id:'wx0123456789abcdef',path:'pages/index'}},'resource',undefined,done)
  expect(navigateToMiniProgram).toHaveBeenCalledTimes(1)
  navigateToMiniProgram.mock.calls[0][0].fail({errMsg:'navigateToMiniProgram:fail cancel'})
  expect(done).toHaveBeenCalledWith('cancel')
})
it('reports clipboard success rather than claiming the browser or service opened',()=>{
  const setClipboardData=vi.fn(),done=vi.fn()
  vi.stubGlobal('uni',{setClipboardData})
  executeServiceChannel({key:'copy',label:'复制链接',action:{kind:'copy_link',url:'https://service.example.com'}},'resource',undefined,done)
  setClipboardData.mock.calls[0][0].success()
  expect(done).toHaveBeenCalledWith('copy_success')
})
it('never accepts a raw H5 URL as a web-view route argument',()=>{
  const navigateTo=vi.fn(),done=vi.fn()
  vi.stubGlobal('uni',{navigateTo})
  executeServiceChannel({key:'web',label:'网页服务',action:{kind:'web_view',url:'https://service.example.com/private?source=school'}},'resource',undefined,done,{version_id:'version',click_id:'click'})
  expect(navigateTo.mock.calls[0][0].url).toBe('/pages/operations/webview?id=resource&channel=web&version=version&click=click')
  expect(navigateTo.mock.calls[0][0].url).not.toContain('https')
})
it('requires an authorized downloaded QR image and never claims a scan occurred',()=>{
  const previewImage=vi.fn(),done=vi.fn()
  vi.stubGlobal('uni',{previewImage})
  const channel={key:'qr',label:'二维码',action:{kind:'qr_code' as const,image_url:'/api/v1/schools/s/services/r/media?channel=qr'}}
  executeServiceChannel(channel,'resource',undefined,done)
  expect(done).toHaveBeenCalledWith('fail');expect(previewImage).not.toHaveBeenCalled()
  executeServiceChannel(channel,'resource','local-image',done)
  previewImage.mock.calls[0][0].success()
  expect(done).toHaveBeenLastCalledWith('qr_displayed')
})
