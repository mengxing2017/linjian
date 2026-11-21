interface Invalidation {
  id: string
  version: number
  content: boolean
  contentVersion: number
  authorId?: string
}
const pending = new Map<string, Map<string, Invalidation>>()
const listeners = new Set<(event: { scope: string; school: string; id: string }) => void>()
export function onPostInvalidated(listener: (event: { scope: string; school: string; id: string }) => void) {
  listeners.add(listener)
  return () => { listeners.delete(listener) }
}
let sequence = 0
const key = (scope: string, school: string) => JSON.stringify([scope, school])
export function invalidatePost(scope: string, school: string, id: string, content = false, authorId?: string) {
  const scopeKey = key(scope, school)
  const items = pending.get(scopeKey) ?? new Map<string, Invalidation>()
  const version = ++sequence
  items.set(id, { id, version, content: content || !!items.get(id)?.content, contentVersion: content ? version : (items.get(id)?.contentVersion ?? 0), authorId: authorId ?? items.get(id)?.authorId })
  pending.set(scopeKey, items)
  listeners.forEach(listener => listener({ scope, school, id }))
}
export function invalidatedPosts(scope: string, school: string) {
  return [...(pending.get(key(scope, school))?.values() ?? [])]
}
export function acknowledgePost(scope: string, school: string, id: string, version: number) {
  const scopeKey = key(scope, school)
  const items = pending.get(scopeKey)
  if (items?.get(id)?.version === version) items.delete(id)
  if (!items?.size) pending.delete(scopeKey)
}
export function clearPostInvalidations() {
  pending.clear()
}
