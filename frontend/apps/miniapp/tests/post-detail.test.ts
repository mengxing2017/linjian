import { expect, it, vi } from 'vitest'
import { createPostDetail } from '../src/features/community/use-post-detail'
import type { CommunityPost, Comment, PostInteractionState } from '../src/features/community/api'

const school = '40000000-0000-0000-0000-000000000001'
const id = '40000000-0000-0000-0000-000000000002'
const snapshot: PostInteractionState = { id, school_id: school, audit_status: 'approved', visibility: 'visible', like_count: 2, comment_count: 1, liked: false, collected: false, author_followed: false, can_interact: true, can_share: true }
const post: CommunityPost = { id, school_id: school, author_id: 'author', author_name: '作者', title: '帖子', display_label: '帖子', content: '正文', images: [], tags: [], audit_status: 'approved', visibility: 'visible', revision: 1, display_published_at: '2026-11-21T00:00:00Z', published_at_text: '2026-11-21 08:00', like_count: 2, comment_count: 1 }
function comment(id = 'comment'): Comment {
  return { id, school_id: school, post_id: post.id, author_id: 'author', author_name: '作者', content: '评论', audit_status: 'approved', visibility: 'visible', deleted: false, parent_id: null, revision: 1, created_at: '2026-11-21T00:00:00Z' }
}
function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>(done => { resolve = done })
  return { promise, resolve }
}
function setup(beforePublicAction?: () => Promise<boolean>) {
  let key = 'account:school:post'
  const api = {
    post: vi.fn().mockResolvedValue(post), interactionState: vi.fn().mockResolvedValue(snapshot),
    comments: vi.fn().mockResolvedValue({ items: [comment()], page: { number: 1, size: 20, has_more: true } }),
    react: vi.fn().mockResolvedValue(null), follow: vi.fn().mockResolvedValue(null),
    comment: vi.fn().mockResolvedValue({ ...comment('new'), audit_status: 'pending' }), deleteComment: vi.fn().mockResolvedValue(null),
  }
  const invalidate = vi.fn(), notify = vi.fn(), confirm = vi.fn().mockResolvedValue(true)
  const detail = createPostDetail({ api, school: () => school, id: () => id, key: () => key, invalidate, notify, confirm, beforePublicAction, failure: cause => cause instanceof Error ? cause.message : '失败' })
  detail.show()
  return { api, detail, invalidate, notify, confirm, changeScope: () => { key = 'another'; detail.reset() } }
}
it('gates new public actions without losing reply drafts or blocking private collection', async () => {
  const gate = vi.fn().mockResolvedValue(false)
  const { detail, api } = setup(gate)
  await detail.load()
  detail.content.value = '保留评论草稿'
  detail.reply(comment())
  await detail.comment()
  await detail.react('like')
  expect(api.comment).not.toHaveBeenCalled()
  expect(api.react).not.toHaveBeenCalled()
  expect(detail.content.value).toBe('保留评论草稿')
  expect(detail.parent.value?.id).toBe('comment')
  await detail.react('collect')
  expect(api.react).toHaveBeenCalledWith(school, id, 'collect', true)
  expect(gate).toHaveBeenCalledTimes(2)
})
it('preserves a missing author title rather than copying the display label into it', async () => {
  const { api, detail } = setup()
  api.post.mockResolvedValue({ ...post, title: '', display_label: '正文' })
  await detail.load()
  expect(detail.post.value?.title).toBe('')
  expect(detail.post.value?.display_label).toBe('正文')
  expect(detail.post.value?.content).toBe('正文')
})
it('bounds comments to one three-page group while keeping earlier comments reachable', async () => {
  const { api, detail } = setup()
  await detail.load()
  api.comments.mockImplementation(async (_school, _post, page = 1) => ({ items: [comment(`page-${page}`)], page: { number: page, size: 20, has_more: true } }))
  await detail.next()
  await detail.next()
  expect(detail.comments.value).toHaveLength(3)
  await detail.next()
  expect(detail.comments.value.map(item => item.id)).toEqual(['page-4'])
  await detail.loadComments(1)
  expect(detail.page.value).toBe(1)
  expect(detail.comments.value.map(item => item.id)).toEqual(['page-1'])
})
it('refreshes only interaction state after likes, preserving drafts, replies and loaded comments', async () => {
  const { api, detail } = setup()
  await detail.load()
  detail.content.value = '未发送草稿'
  detail.reply(comment())
  api.comments.mockResolvedValueOnce({ items: [comment('second')], page: { number: 2, size: 20, has_more: false } })
  await detail.next()
  api.interactionState.mockResolvedValueOnce({ ...snapshot, liked: true, like_count: 3 })
  await detail.react('like')
  expect(api.react).toHaveBeenCalledWith(school, id, 'like', true)
  expect(api.post).toHaveBeenCalledOnce()
  expect(detail.post.value?.like_count).toBe(3)
  expect(detail.content.value).toBe('未发送草稿')
  expect(detail.parent.value?.id).toBe('comment')
  expect(detail.comments.value.map(item => item.id)).toEqual(['comment', 'second'])
  expect(detail.more.value).toBe(false)
})
it('does not increment counts for pending comments and resets pagination only after confirmed sends', async () => {
  const { api, detail, notify, invalidate } = setup()
  await detail.load()
  detail.content.value = ' 回复 '
  detail.reply(comment())
  await detail.comment()
  expect(api.comment).toHaveBeenCalledWith(school, id, '回复', 'comment')
  expect(detail.content.value).toBe('')
  expect(detail.parent.value).toBeUndefined()
  expect(detail.post.value?.comment_count).toBe(1)
  expect(notify).toHaveBeenCalledWith('已提交，等待审核')
  expect(invalidate).toHaveBeenCalledOnce()
  expect(api.comments).toHaveBeenLastCalledWith(school, id, 1)
  expect(api.post).toHaveBeenCalledOnce()
})
it('retains failed or ambiguous drafts and never automatically repeats comment creation', async () => {
  const { api, detail } = setup()
  await detail.load()
  detail.content.value = '保留草稿'
  detail.reply(comment())
  api.comment.mockRejectedValue(Object.assign(new Error('结果待确认'), { status: 0 }))
  await detail.comment()
  expect(api.comment).toHaveBeenCalledOnce()
  expect(detail.content.value).toBe('保留草稿')
  expect(detail.parent.value?.id).toBe('comment')
  expect(detail.error.value).toContain('待确认')
})
it('distinguishes committed writes from a subsequent snapshot failure', async () => {
  const { api, detail, invalidate } = setup()
  await detail.load()
  detail.content.value = '已发布'
  api.interactionState.mockRejectedValue(new Error('同步不可用'))
  await detail.comment()
  expect(detail.content.value).toBe('')
  expect(invalidate).toHaveBeenCalledOnce()
  expect(detail.error.value).toContain('操作已完成，状态同步失败')
  expect(detail.canShare.value).toBe(false)
})
it('blocks duplicate sends and does not erase a newer draft after completion', async () => {
  const { api, detail } = setup()
  await detail.load()
  const pending = deferred<Comment>()
  api.comment.mockReturnValue(pending.promise)
  detail.content.value = '原草稿'
  const sending = detail.comment()
  await detail.comment()
  detail.content.value = '新的草稿'
  pending.resolve(comment('sent'))
  await sending
  expect(api.comment).toHaveBeenCalledOnce()
  expect(detail.content.value).toBe('新的草稿')
})
it('discards an old pagination response when a successful comment resets the first page', async () => {
  const { api, detail } = setup()
  await detail.load()
  const pending = deferred<{ items: Comment[]; page: { number: number; size: number; has_more: boolean } }>()
  api.comments.mockReturnValueOnce(pending.promise)
  const next = detail.next()
  detail.content.value = '新评论'
  await detail.comment()
  pending.resolve({ items: [comment('stale-page')], page: { number: 2, size: 20, has_more: false } })
  await next
  expect(detail.comments.value.map(item => item.id)).toEqual(['comment'])
  expect(detail.more.value).toBe(true)
})
it('does not apply a former account response or clear a new account draft', async () => {
  const { api, detail, changeScope, notify, invalidate } = setup()
  await detail.load()
  const pending = deferred<Comment>()
  api.comment.mockReturnValueOnce(pending.promise)
  detail.content.value = '旧账号'
  const sending = detail.comment()
  changeScope()
  detail.content.value = '新账号'
  pending.resolve(comment())
  await sending
  expect(detail.content.value).toBe('新账号')
  expect(notify).not.toHaveBeenCalled()
  expect(invalidate).not.toHaveBeenCalled()
  expect(detail.post.value).toBeUndefined()
})
it('never restores sharing while hidden or after disposal, even when a write finishes', async () => {
  const { api, detail } = setup()
  await detail.load()
  const pending = deferred<null>()
  api.react.mockReturnValueOnce(pending.promise)
  const liking = detail.react('like')
  detail.hide()
  pending.resolve(null)
  await liking
  expect(detail.canShare.value).toBe(false)
  detail.show()
  await detail.resume()
  expect(detail.canShare.value).toBe(true)
  detail.dispose()
  await detail.resume()
  expect(detail.canShare.value).toBe(false)
})
it('rejects unusable reply targets and retains drafts on revision conflicts', async () => {
  const { api, detail } = setup()
  await detail.load()
  detail.reply({ ...comment(), visibility: 'hidden' })
  expect(detail.parent.value).toBeUndefined()
  detail.content.value = '草稿'
  api.deleteComment.mockRejectedValue(Object.assign(new Error('版本冲突'), { status: 409 }))
  await detail.removeComment(comment())
  expect(detail.content.value).toBe('草稿')
  expect(detail.error.value).toContain('版本冲突')
  expect(api.comments).toHaveBeenLastCalledWith(school, id, 1)
})
it('uses server capabilities rather than local publication state to enable interactions', async () => {
  const { api, detail } = setup()
  api.interactionState.mockResolvedValue({ ...snapshot, can_interact: false, can_share: false })
  await detail.load()
  expect(detail.canInteract.value).toBe(false)
  expect(detail.canShare.value).toBe(false)
  await detail.react('like')
  expect(api.react).not.toHaveBeenCalled()
})
it('coalesces initial loads and permits correction after a rejected comment without reloading the post', async () => {
  const { api, detail } = setup()
  await Promise.all([detail.load(), detail.load()])
  expect(api.post).toHaveBeenCalledOnce()
  detail.content.value = '需修改的评论'
  api.comment.mockRejectedValueOnce(Object.assign(new Error('内容包含禁止发布的词语'), { status: 400 }))
  await detail.comment()
  expect(detail.content.value).toBe('需修改的评论')
  expect(detail.canInteract.value).toBe(true)
  detail.content.value = '修改后的评论'
  await detail.comment()
  expect(api.comment).toHaveBeenCalledTimes(2)
  expect(api.post).toHaveBeenCalledOnce()
})
it('does not allow an older resume snapshot to overwrite the newest one', async () => {
  const { api, detail } = setup()
  await detail.load()
  const old = deferred<PostInteractionState>()
  api.interactionState.mockReturnValueOnce(old.promise)
  const resuming = detail.resume()
  api.interactionState.mockResolvedValueOnce({ ...snapshot, like_count: 4, liked: true })
  await detail.resume()
  old.resolve({ ...snapshot, like_count: 1 })
  await resuming
  expect(detail.interaction.value?.like_count).toBe(4)
  expect(detail.interaction.value?.liked).toBe(true)
})
it('does not restore comments from a parallel read after the parent becomes hidden', async () => {
  const { api, detail } = setup()
  await detail.load()
  detail.content.value = '草稿'
  api.interactionState.mockResolvedValueOnce({ ...snapshot, visibility: 'hidden', can_interact: false, can_share: false })
  await detail.comment()
  expect(detail.comments.value).toEqual([])
  expect(detail.canShare.value).toBe(false)
  expect(detail.more.value).toBe(false)
})
