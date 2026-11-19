export function formatCoordinates(
  latitude: number | null | undefined,
  longitude: number | null | undefined,
): string {
  if (latitude === null || latitude === undefined) return '—'
  if (longitude === null || longitude === undefined) return '—'
  if (!Number.isFinite(latitude) || !Number.isFinite(longitude)) return '—'
  return `${trimCoordinate(latitude)}, ${trimCoordinate(longitude)}`
}

function trimCoordinate(value: number): string {
  return String(Number(value.toFixed(6)))
}
