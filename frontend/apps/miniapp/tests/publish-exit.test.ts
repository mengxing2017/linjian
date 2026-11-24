import { afterEach, expect, it, vi } from 'vitest'
import { createPublishExit } from '../src/features/community/publish-exit'

function setup(tapIndex = 0) {
  let text = '', busy = false, scope = 'initial'
  const save = vi.fn().mockReturnValue(true), leave = vi.fn().mockResolvedValue(undefined)
  const discard = vi.fn(() => { text = '' })
  vi.stubGlobal('uni', { showActionSheet: vi.fn().mockResolvedValue({ tapIndex }), showToast: vi.fn() })
  const exit = createPublishExit({ snapshot: () => text, save, discard, busy: () => busy, scope: () => scope, leave })
  return { exit, save, leave, discard, edit: (value: string) => { text = value }, busy: () => { busy = true }, changeScope: () => { scope = 'changed' } }
}
afterEach(() => vi.unstubAllGlobals())
it('leaves an unchanged editor without a prompt', async () => {
  const { exit, leave } = setup()
  await exit.cancel()
  expect(uni.showActionSheet).not.toHaveBeenCalled()
  expect(leave).toHaveBeenCalledOnce()
})
it('continues editing or saves before leaving, and refuses to lose a failed save', async () => {
  const continued = setup(0)
  continued.edit('draft'); await continued.exit.cancel()
  expect(continued.leave).not.toHaveBeenCalled()
  const saved = setup(1)
  saved.edit('draft'); await saved.exit.cancel()
  expect(saved.save).toHaveBeenCalledOnce(); expect(saved.leave).toHaveBeenCalledOnce()
  saved.exit.saveOnHide()
  expect(saved.save).toHaveBeenCalledOnce()
  saved.edit('changed draft')
  saved.save.mockReturnValue(false)
  await saved.exit.cancel()
  expect(saved.leave).toHaveBeenCalledOnce()
})
it('does not resurrect discarded drafts on hide, but autosaves later edits', async () => {
  const { exit, edit, discard, save, leave } = setup(2)
  edit('draft'); await exit.cancel(); exit.saveOnHide()
  expect(discard).toHaveBeenCalledOnce(); expect(save).not.toHaveBeenCalled(); expect(leave).toHaveBeenCalledOnce()
  edit('new draft'); exit.saveOnHide()
  expect(save).toHaveBeenCalledOnce()
})
it('does not recreate a submitted draft and blocks exits while busy', async () => {
  const { exit, edit, save, busy, leave } = setup(1)
  edit(''); exit.checkpoint(); exit.saveOnHide()
  expect(save).not.toHaveBeenCalled()
  edit('uploading'); busy(); await exit.cancel()
  expect(uni.showActionSheet).not.toHaveBeenCalled(); expect(leave).not.toHaveBeenCalled()
})
it('safely autosaves system hides without opening an exit dialog', () => {
  const { exit, edit, save } = setup()
  edit('draft'); exit.saveOnHide()
  expect(save).toHaveBeenCalledOnce(); expect(uni.showActionSheet).not.toHaveBeenCalled()
})
it('does not repeat successful autosaves and retries a failed hide save', () => {
  const { exit, edit, save } = setup()
  exit.saveOnHide()
  expect(save).not.toHaveBeenCalled()
  edit('new text')
  save.mockReturnValueOnce(false)
  expect(exit.saveOnHide()).toBe(false)
  expect(exit.saveOnHide()).toBe(true)
  exit.saveOnHide()
  expect(save).toHaveBeenCalledTimes(2)
})
it('does not discard a new scope after an old action sheet resolves', async () => {
  const { exit, edit, discard, save, leave, changeScope } = setup(2)
  edit('old school draft')
  vi.mocked(uni.showActionSheet).mockImplementationOnce(async () => { changeScope(); return { tapIndex: 2 } })
  await exit.cancel()
  expect(discard).not.toHaveBeenCalled(); expect(save).not.toHaveBeenCalled(); expect(leave).not.toHaveBeenCalled()
})
