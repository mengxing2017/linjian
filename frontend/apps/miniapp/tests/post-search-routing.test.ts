import { expect, it } from 'vitest'
import { postSearchRoute } from '../src/features/community/search-route'

it('keeps personal content routes and falls back to ordinary search for unknown modes', () => {
  expect(postSearchRoute('mine')).toEqual({ view: 'mine', title: '我的发布' })
  expect(postSearchRoute('collect')).toEqual({ view: 'collect', title: '我的收藏' })
  for (const view of [undefined, '', 'following', 'admin', ['mine']]) {
    expect(postSearchRoute(view)).toEqual({ view: 'search', title: '搜索帖子' })
  }
})
