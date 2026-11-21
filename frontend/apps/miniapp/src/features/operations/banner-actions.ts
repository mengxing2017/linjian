export async function executeBannerAction(action: unknown) {
  if (!action || typeof action !== 'object' || !('kind' in action)) throw new Error('广告动作不可用')
  if (action.kind === 'none') return
  if (action.kind !== 'internal_page' || !('page' in action)) throw new Error('首页广告只允许站内访问')
  switch (action.page) {
    case 'feed': await uni.switchTab({ url: '/pages/index/index' }); return
    case 'resources': await uni.navigateTo({ url: '/pages/index/category' }); return
    case 'legal': await uni.navigateTo({ url: '/pages/user/legal' }); return
    case 'post': {
      if (!('object_id' in action) || typeof action.object_id !== 'string' || !/^[0-9a-f-]{36}$/i.test(action.object_id)) throw new Error('帖子目标无效')
      await uni.navigateTo({ url: `/pages/index/detail?id=${encodeURIComponent(action.object_id)}` }); return
    }
    default: throw new Error('站内目标不可用')
  }
}
