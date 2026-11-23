import { expect, it, vi } from 'vitest'
import { createClient } from '../src/http/client'
import { createCommunityClient, normalizePostContent, postFieldError } from '../src/modules/community'
import fixture from '../fixtures/post-untitled.json'

it('allows omitted titles but requires meaningful body text using scalar limits', () => {
  expect(normalizePostContent({ content: ' 正文\n👩‍💻 ' })).toEqual({ title: '', content: '正文\n👩‍💻' })
  expect(normalizePostContent({ title: ' \n ', content: '正文' }).title).toBe('')
  expect(postFieldError('title', '😀'.repeat(200))).toBeUndefined()
  expect(postFieldError('title', '😀'.repeat(201))).toBe('标题最多 200 字')
  expect(postFieldError('content', '😀'.repeat(20000))).toBeUndefined()
  expect(postFieldError('content', '😀'.repeat(20001))).toBe('正文最多 20000 字')
  expect(postFieldError('title', '两行\n标题')).toBe('标题必须为单行文本')
  expect(normalizePostContent({ title: '\u0085 标题 \u0085', content: '\u0085 正文 \u0085' })).toEqual({ title: '标题', content: '正文' })
  expect(postFieldError('title', '\ufeff')).toBe('标题不能仅包含不可见字符')
  for (const content of ['', ' \n ', '\u200b\u200d\ufe0f']) {
    expect(() => normalizePostContent({ title: '仅标题', content })).toThrow('请填写正文')
  }
})

it('uses distinct creation and complete-update commands without generating a title', async () => {
  const transport = vi.fn().mockResolvedValue({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body: fixture })
  const client = createCommunityClient(createClient(transport))
  const created = await client.create('school', { content: ' 正文 ', images: [], tags: [] })
  expect(created.title).toBe('')
  expect(transport.mock.calls[0][0]).toMatchObject({ method: 'POST' })
  expect(JSON.parse(transport.mock.calls[0][0].body).data.attributes).toEqual({ title: '', content: '正文', images: [], tags: [] })
  await client.update('school', 'post', { title: '', content: '新正文', images: [], tags: [], revision: 3 })
  expect(transport.mock.calls[1][0]).toMatchObject({ method: 'PUT', path: '/api/v1/schools/school/posts/post' })
  expect(JSON.parse(transport.mock.calls[1][0].body).data.attributes).toMatchObject({ title: '', revision: 3 })
})
