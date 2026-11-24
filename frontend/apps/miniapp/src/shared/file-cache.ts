interface CachedFile { path: string; bytes: number }
interface Entry {
  promise: Promise<string>
  owners: Set<object>
  file?: CachedFile
}

export function createFileCache(remove: (path: string) => void, maxEntries = 48, maxBytes = 32 * 1024 * 1024) {
  const entries = new Map<string, Entry>()
  let generation = 0
  function prune(protectedEntry?: Entry) {
    let bytes = 0
    for (const entry of entries.values()) bytes += entry.file?.bytes ?? 0
    for (const [key, entry] of entries) {
      if (entries.size <= maxEntries && bytes <= maxBytes) break
      if (!entry.file || entry.owners.size || entry === protectedEntry) continue
      entries.delete(key)
      bytes -= entry.file.bytes
      remove(entry.file.path)
    }
  }
  function read(key: string, load: () => Promise<CachedFile>, owner?: object): Promise<string> {
    const existing = entries.get(key)
    if (existing) {
      entries.delete(key)
      entries.set(key, existing)
      if (owner) existing.owners.add(owner)
      return existing.promise
    }
    const turn = generation
    const entry: Entry = { promise: Promise.resolve(''), owners: new Set(owner ? [owner] : []) }
    entry.promise = load().then(file => {
      if (turn !== generation) {
        remove(file.path)
        throw new Error('资源范围已改变')
      }
      entry.file = file
      prune(entry)
      return file.path
    }).catch(error => {
      if (entries.get(key) === entry) entries.delete(key)
      throw error
    })
    entries.set(key, entry)
    return entry.promise
  }
  function release(owner: object) {
    for (const entry of entries.values()) entry.owners.delete(owner)
    prune()
  }
  function clear() {
    generation++
    for (const entry of entries.values()) if (entry.file) remove(entry.file.path)
    entries.clear()
  }
  return { read, release, clear }
}
