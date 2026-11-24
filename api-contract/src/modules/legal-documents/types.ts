export type LegalDocumentKind = 'user_agreement' | 'privacy_policy'
export interface LegalSummary {
  id: LegalDocumentKind
  kind: LegalDocumentKind
  revision: number
  updated_at: string
}
export interface LegalDocument extends LegalSummary { content: string | null; updated_by_name?: string | null }
export interface LegalRevision { id: string; revision: number; content: string | null; updated_at: string; updated_by_name: string }
export const legalTitles: Record<LegalDocumentKind, string> = { user_agreement: '用户协议', privacy_policy: '隐私政策' }
export function validLegalContent(content: string) {
  return Array.from(content).length <= 50000 && content.replace(/[\p{White_Space}\u200b-\u200f\u2060\ufeff]/gu, '').length > 0
}
