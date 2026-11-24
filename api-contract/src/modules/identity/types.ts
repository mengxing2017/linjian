import type { User } from '../business/types'
import type { LegalSummary, LegalDocumentKind } from '../legal-documents/types'
export type { Session, User } from '../business/types'
export interface MiniappSession {
  id: string
  token: string
  user: User
}
export interface LegalAcceptance { kind: LegalDocumentKind; revision: number }
export interface OnboardingSchool { id: string; name: string; short_name: string; province: string; city: string }
export interface OnboardingProgress {
  id: string
  state: 'consent_required' | 'school_required'
  ticket: string
  expires_at: string
  documents: LegalSummary[]
  accepted: LegalAcceptance[]
  first_login: boolean
  school_id: string | null
}
export interface OnboardingReady extends MiniappSession { state: 'ready' }
export type OnboardingResult = OnboardingProgress | OnboardingReady
export interface OnboardingInput { ticket: string; acceptances?: LegalAcceptance[]; school_id?: string; request_key?: string }
