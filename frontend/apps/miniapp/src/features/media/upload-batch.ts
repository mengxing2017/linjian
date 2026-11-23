export async function uploadBatch<Item>(
  items: Item[],
  send: (item: Item) => Promise<boolean>,
  current: () => boolean,
) {
  let success = 0,
    failure = 0
  for (const item of items) {
    if (!current()) return
    const accepted = await send(item)
    if (!current()) return
    if (accepted) success++
    else failure++
  }
  if (!items.length || !current()) return
  return `本次已上传 ${success} 张${failure ? `，失败 ${failure} 张` : ''}${success ? '，提交表单后生效' : '，请重试失败图片'}`
}
