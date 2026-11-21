import { createClient } from '../../features/identity/api/contract'
import { createUniTransport } from '../transport'
export const client = createClient(
  createUniTransport(import.meta.env.VITE_LINJIAN_API_BASE_URL ?? ''),
)
