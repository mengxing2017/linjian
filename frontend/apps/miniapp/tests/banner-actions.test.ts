import { afterEach, expect, it, vi } from 'vitest'
import { executeBannerAction } from '../src/features/operations/banner-actions'
afterEach(() => vi.unstubAllGlobals())
it('rejects external banner actions even if a malformed response reaches the client', async () => {
  const navigateTo=vi.fn(),setClipboardData=vi.fn(),navigateToMiniProgram=vi.fn()
  vi.stubGlobal('uni',{navigateTo,setClipboardData,navigateToMiniProgram})
  for(const action of [{kind:'external_link',url:'https://example.com'},{kind:'copy_link',url:'https://example.com'},{kind:'mini_program',app_id:'wx0123456789abcdef'},{kind:'internal_page',page:'https://example.com'}]) {
    await expect(executeBannerAction(action)).rejects.toThrow()
  }
  expect(navigateTo).not.toHaveBeenCalled();expect(setClipboardData).not.toHaveBeenCalled();expect(navigateToMiniProgram).not.toHaveBeenCalled()
})
it('keeps the existing internal banner navigation available', async () => {
  const switchTab=vi.fn().mockResolvedValue({}),navigateTo=vi.fn().mockResolvedValue({})
  vi.stubGlobal('uni',{switchTab,navigateTo})
  await executeBannerAction({kind:'internal_page',page:'feed'})
  await executeBannerAction({kind:'internal_page',page:'resources'})
  expect(switchTab).toHaveBeenCalledWith({url:'/pages/index/index'})
  expect(navigateTo).toHaveBeenCalledWith({url:'/pages/index/category'})
})
