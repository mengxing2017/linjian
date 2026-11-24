import { afterEach, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import type { HomeMenuTag } from '../src/features/community/api'
import * as tags from '../src/features/community/publish-tags'
import { component, findAll, mount, tap, text } from './support/vue-component'

const picker = component(new URL('../src/features/community/PublishTagPicker.vue', import.meta.url), { './publish-tags': tags })
const candidates: HomeMenuTag[] = Array.from({ length: 12 }, (_, index) => ({ id: `tag-${index}`, name: `标签${index}`, menu_status: index ? 'shown' : 'hot', source_type: 'admin', enable_status: 'enabled', sort_order: index, revision: 1, created_at: '' }))
afterEach(() => vi.unstubAllGlobals())
function setup(selection = tags.emptyPublishTags()) {
  vi.stubGlobal('uni', { hideKeyboard: vi.fn() })
  const confirmed = vi.fn(), closed = vi.fn()
  const mounted = mount(picker, { open: true, candidates, selection, onConfirm: confirmed, onClose: closed })
  const button = (label: string) => findAll(mounted.root, target => target.type === 'button' && text(target) === label)[0]
  const option = (index: number) => findAll(mounted.root, target => target.type === 'button' && String(target.props['aria-label']).startsWith(`标签${index}，`))[0]
  return { mounted, confirmed, closed, button, option }
}

it('keeps temporary changes separate, cancels on native back, and resets on reopen', async () => {
  const selection = tags.emptyPublishTags()
  const { mounted, confirmed, closed, button, option } = setup(selection)
  tap(option(0)); await nextTick()
  expect(selection.selected).toEqual([])
  expect(text(mounted.root)).toContain('已选 1/10')
  tap(button('取消'))
  expect(closed).toHaveBeenCalledOnce()
  expect(confirmed).not.toHaveBeenCalled()
  mounted.props.open = false; await nextTick()
  mounted.props.open = true; await nextTick()
  expect(text(mounted.root)).toContain('已选 0/10')
  const native = findAll(mounted.root, target => target.type === 'page-container')[0]
  ;(native.props.onBeforeleave as () => void)()
  expect(closed).toHaveBeenCalledTimes(2)
  mounted.unmount()
})

it('searches loaded candidates without losing selection or offering creation', async () => {
  const { mounted, confirmed, button, option } = setup()
  tap(option(0)); await nextTick()
  const input = findAll(mounted.root, target => target.type === 'input')[0]
  ;(input.props.onInput as (event: unknown) => void)({ detail: { value: '不存在' } })
  await nextTick()
  expect(text(mounted.root)).toContain('未找到标签')
  expect(text(mounted.root)).not.toContain('创建标签')
  tap(button('确定'))
  expect(confirmed).toHaveBeenCalledWith({ selected: [{ id: 'tag-0', name: '标签0' }], retained: [], unresolved: [] })
  mounted.unmount()
})

it('limits total choices to ten but still permits deselection', async () => {
  const { mounted, confirmed, button, option } = setup()
  for (let index = 0; index < 10; index++) tap(option(index))
  await nextTick()
  tap(option(10)); await nextTick()
  expect(text(mounted.root)).toContain('最多选择 10 个标签')
  tap(option(0)); tap(option(10)); await nextTick()
  tap(button('确定'))
  expect(confirmed.mock.calls[0][0].selected.map((tag: { id: string }) => tag.id)).toEqual([...candidates.slice(1, 10), candidates[10]].map(tag => tag.id))
  mounted.unmount()
})

it('requires explicit legacy draft cleanup and supports historical removal without candidates', async () => {
  const { mounted, confirmed, button } = setup({ selected: [], retained: ['旧标签'], unresolved: ['自定义'] })
  mounted.props.candidates = []; await nextTick()
  expect(button('确定').props.disabled).toBe(true)
  tap(button('确定')); expect(confirmed).not.toHaveBeenCalled()
  tap(button('自定义 · 确认移除')); await nextTick()
  tap(button('确定'))
  expect(confirmed).toHaveBeenLastCalledWith({ selected: [], retained: ['旧标签'], unresolved: [] })
  tap(button('旧标签 · 移除')); await nextTick()
  tap(button('确定'))
  expect(confirmed).toHaveBeenLastCalledWith(tags.emptyPublishTags())
  mounted.unmount()
})
