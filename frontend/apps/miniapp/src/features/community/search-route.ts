export function postSearchRoute(view: unknown) {
  if (view === 'mine') return { view: 'mine', title: '我的发布' } as const
  if (view === 'collect') return { view: 'collect', title: '我的收藏' } as const
  return { view: 'search', title: '搜索帖子' } as const
}
