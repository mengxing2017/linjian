import type { User } from '../business/types'
export type { Session, User } from '../business/types'
export interface MiniappSession {
  id: string
  token: string
  user: User
}
