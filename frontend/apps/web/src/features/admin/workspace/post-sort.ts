/** Permute only this page; the server owns weights and canonical page boundaries. */
export function movePost<T extends { id: string }>(
  rows: readonly T[],
  id: string,
  destination: number,
): T[] {
  const index = rows.findIndex((row) => row.id === id)
  if (index < 0 || destination < 0 || destination >= rows.length) return [...rows]
  const result = [...rows]
  const [row] = result.splice(index, 1)
  result.splice(destination, 0, row)
  return result
}
