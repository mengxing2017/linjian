export const PAGES_PER_WINDOW = 3

export function windowStart(page: number) {
  return Math.floor((Math.max(1, page) - 1) / PAGES_PER_WINDOW) * PAGES_PER_WINDOW + 1
}

export function previousWindow(page: number) {
  return Math.max(1, windowStart(page) - PAGES_PER_WINDOW)
}

export function mergePageWindow<Item extends { id: string }>(existing: Item[], page: number, incoming: Item[], next: number) {
  const retained = next === page + 1 && windowStart(page) === windowStart(next) ? existing : []
  const merged = new Map(retained.map(item => [item.id, item]))
  for (const item of incoming) merged.set(item.id, item)
  return [...merged.values()]
}
