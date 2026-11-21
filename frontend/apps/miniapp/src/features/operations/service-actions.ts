import type { EffectiveChannel, ActionOutcome } from './api'
export function executeServiceChannel(channel: EffectiveChannel, service: string, qrPath: string | undefined, done: (outcome: ActionOutcome) => void, context?: {version_id:string;click_id:string}) {
  const failure = (error: { errMsg?: string }) => done(error.errMsg?.toLowerCase().includes('cancel') ? 'cancel' : 'fail')
  try { switch (channel.action.kind) {
    case 'mini_program':
      uni.navigateToMiniProgram({ appId: channel.action.app_id, path: channel.action.path, envVersion: 'release', success: () => done('api_success'), fail: failure })
      break
    case 'web_view':
      if(!context) {done('fail');return}
      uni.navigateTo({ url: `/pages/operations/webview?id=${encodeURIComponent(service)}&channel=${encodeURIComponent(channel.key)}&version=${encodeURIComponent(context.version_id)}&click=${encodeURIComponent(context.click_id)}`, success: () => done('api_success'), fail: failure })
      break
    case 'copy_link':
      uni.setClipboardData({ data: channel.action.url, success: () => done('copy_success'), fail: () => done('fail') })
      break
    case 'qr_code':
      if (!qrPath) { done('fail'); return }
      uni.previewImage({ urls: [qrPath], success: () => done('qr_displayed'), fail: () => done('fail') })
      break
  } } catch { done('fail') }
}
