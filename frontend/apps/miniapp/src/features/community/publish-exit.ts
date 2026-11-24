export function createPublishExit(options: {
  snapshot: () => string | number
  save: () => boolean
  discard: () => void
  busy: () => boolean
  scope: () => string
  leave: () => Promise<unknown>
}) {
  let baseline = options.snapshot()
  let suppressed: string | number = baseline
  let leaving = false
  function checkpoint() { baseline = options.snapshot(); suppressed = baseline }
  function saveOnHide() {
    const snapshot = options.snapshot()
    if (suppressed === snapshot) return true
    if (!options.save()) return false
    suppressed = snapshot
    return true
  }
  async function cancel() {
    if (leaving) return
    if (options.busy()) {
      uni.showToast({ title: '正在处理，请稍候', icon: 'none' })
      return
    }
    leaving = true
    const scope = options.scope()
    try {
      if (options.snapshot() !== baseline) {
        const choice = await uni.showActionSheet({ itemList: ['继续编辑', '保存草稿并退出', '放弃修改并退出'] })
        if (scope !== options.scope()) return
        if (choice.tapIndex === 0) return
        if (choice.tapIndex === 1) {
          if (!options.save()) return
          checkpoint()
        } else if (choice.tapIndex === 2) { options.discard(); checkpoint() }
        else return
      }
      await options.leave()
    } catch {
      return
    } finally {
      leaving = false
    }
  }
  return { checkpoint, saveOnHide, cancel }
}
