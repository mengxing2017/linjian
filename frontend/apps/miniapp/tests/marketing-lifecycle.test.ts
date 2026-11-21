import { expect, it, vi } from 'vitest'
import { rewardLifecycle, type RewardedVideo } from '../src/features/operations/marketing-lifecycle'
function setup() {
  let closed: (result: { isEnded?: boolean }) => void = () => {}
  const ad: RewardedVideo = { show: vi.fn().mockResolvedValue(undefined), load: vi.fn().mockResolvedValue(undefined), destroy: vi.fn(), onLoad: vi.fn(), offLoad: vi.fn(), onError: vi.fn(), offError: vi.fn(), onClose: listener => { closed = listener }, offClose: vi.fn() }
  const callbacks = { current: vi.fn().mockReturnValue(true), record: vi.fn(), completed: vi.fn(), failed: vi.fn(), closed: vi.fn() }
  const lifecycle = rewardLifecycle(ad, callbacks)
  return { ad, callbacks, lifecycle, close: (result: { isEnded?: boolean }) => closed(result) }
}
it('applies the local-only scene once for a full completion, never for duplicate callbacks',async()=>{
  const { ad,callbacks,lifecycle,close }=setup()
  const attempt=lifecycle.show()
  expect(ad.show).toHaveBeenCalledTimes(1)
  await attempt
  close({isEnded:true});close({isEnded:true})
  expect(callbacks.completed).toHaveBeenCalledTimes(1)
  expect(callbacks.record.mock.calls.map(([stage])=>stage)).toEqual(['show_api_success','client_completed'])
  expect(await lifecycle.show()).toBe(false)
})
it('does not grant on early or ambiguous closure',async()=>{
  const { callbacks,lifecycle,close }=setup()
  await lifecycle.show();close({isEnded:false});close({isEnded:true})
  expect(callbacks.completed).not.toHaveBeenCalled()
  expect(callbacks.record).toHaveBeenCalledWith('closed_early')
  await lifecycle.show();close({})
  expect(callbacks.completed).not.toHaveBeenCalled()
})
it('prefetches after failure but never automatically opens a second ad',async()=>{
  const { ad,callbacks,lifecycle }=setup()
  vi.mocked(ad.show).mockRejectedValue(new Error('not ready'))
  expect(await lifecycle.show()).toBe(false)
  expect(ad.show).toHaveBeenCalledTimes(1);expect(ad.load).toHaveBeenCalledTimes(1)
  expect(callbacks.completed).not.toHaveBeenCalled()
})
it('detaches listeners and ignores completion after page disposal or school changes',async()=>{
  const { ad,callbacks,lifecycle,close }=setup()
  await lifecycle.show();lifecycle.dispose();lifecycle.dispose();close({isEnded:true})
  expect(ad.destroy).toHaveBeenCalledTimes(1);expect(ad.offClose).toHaveBeenCalledTimes(1)
  expect(callbacks.completed).not.toHaveBeenCalled()
  const changed=setup();await changed.lifecycle.show();changed.callbacks.current.mockReturnValue(false);changed.close({isEnded:true})
  expect(changed.callbacks.completed).not.toHaveBeenCalled()
})
