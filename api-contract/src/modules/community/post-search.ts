export function normalizePostSearch(input = ''): string {
  const keyword = input.trim()
  if (Array.from(keyword).length > 100) throw new Error('搜索关键词最多 100 字')
  return keyword
}
