import { requestResource } from '../../http/browser-client'
export interface SystemInfo {
  id: string
  application: 'linjian'
  version: string
  stage: 'business'
}
export function getSystemInfo(signal?: AbortSignal): Promise<SystemInfo> {
  return requestResource('/api/v1/system', 'GET', undefined, signal)
}
