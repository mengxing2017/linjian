export async function mapConcurrent<Input, Output>(items: readonly Input[], read: (item: Input) => Promise<Output>, current: () => boolean = () => true, concurrency = 3) {
  let index = 0
  const results: Output[] = []
  async function worker() {
    while (index < items.length && current()) {
      const position = index++
      results[position] = await read(items[position])
    }
  }
  await Promise.all(Array.from({ length: Math.min(Math.max(1, concurrency), items.length) }, worker))
  return results
}
