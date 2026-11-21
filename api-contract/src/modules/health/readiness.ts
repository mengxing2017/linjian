import { getJson } from '../../http/browser-client'

export interface ReadyResponse {
  status: 'ready'
}

export function getReadiness(signal?: AbortSignal): Promise<ReadyResponse> {
  return getJson('/readyz', signal)
}
