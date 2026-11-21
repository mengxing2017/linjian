import { decodeResource, invalidDocument, object, type ResourceDocument, type ResourceObject } from '../../http/protocol'
import type { SchoolChangeDocument, SchoolChangePolicy, SchoolChangeRequest, SchoolChangeSummary } from './types'

const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
const states = new Set(['pending', 'approved', 'rejected', 'cancelled', 'invalidated'])
const date = (value: unknown) => typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T/.test(value) && Number.isFinite(Date.parse(value))
const nullableString = (value: unknown) => value === null || typeof value === 'string'
const integer = (value: unknown) => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0
function resource(value: ResourceObject, type: string) {
  if (value.type !== type || !uuid.test(value.id)) return invalidDocument()
  return decodeResource(value)
}
export function readSummary(value: ResourceObject): SchoolChangeSummary {
  const result = resource(value, 'miniapp-school-change-requests')
  for (const key of ['account_id', 'from_school_id', 'to_school_id'])
    if (typeof result[key] !== 'string' || !uuid.test(result[key] as string)) return invalidDocument()
  if (result.from_school_id === result.to_school_id || typeof result.status !== 'string' || !states.has(result.status) || !integer(result.revision) || Number(result.revision) < 1) return invalidDocument()
  for (const key of ['from_school_name', 'to_school_name', 'consent_version']) if (typeof result[key] !== 'string') return invalidDocument()
  if (!date(result.submitted_at) || !date(result.review_due_at) || (result.closed_at !== null && !date(result.closed_at)) || (result.reviewed_at !== null && !date(result.reviewed_at))) return invalidDocument()
  if (result.reviewed_by !== null && (typeof result.reviewed_by !== 'string' || !uuid.test(result.reviewed_by))) return invalidDocument()
  const automatic = result.result_reason_code === 'review_timeout'
  const manual = result.status === 'approved' || (result.status === 'rejected' && !automatic)
  if ((result.status === 'pending') !== (result.closed_at === null) || (automatic && result.status !== 'rejected')
    || (manual ? result.reviewed_by === null || result.reviewed_at === null : result.reviewed_by !== null || result.reviewed_at !== null)) return invalidDocument()
  if (!nullableString(result.result_reason_code) || typeof result.personal_data_purged !== 'boolean' || !Array.isArray(result.blocked_reasons) || result.blocked_reasons.some(reason => typeof reason !== 'string')) return invalidDocument()
  return result as unknown as SchoolChangeSummary
}
export function readRequest(document: ResourceDocument, nullable = false): SchoolChangeRequest | null {
  if (document.data === null && nullable) return null
  if (document.data === null || Array.isArray(document.data)) return invalidDocument()
  const summary = readSummary(document.data)
  const result = decodeResource(document.data)
  if (result.student_relationship !== null && !['enrolled', 'new_student'].includes(String(result.student_relationship))) return invalidDocument()
  for (const key of ['declared_student_name', 'declared_student_number', 'reason', 'result_reason']) if (!nullableString(result[key])) return invalidDocument()
  if (!Array.isArray(result.documents)) return invalidDocument()
  const ids = new Set<string>()
  for (const document of result.documents) {
    if (!object(document) || typeof document.id !== 'string' || !uuid.test(document.id) || ids.has(document.id) || document.mime !== 'image/png' || typeof document.available !== 'boolean') return invalidDocument()
    ids.add(document.id)
  }
  if (result.review_guidance !== undefined && typeof result.review_guidance !== 'string') return invalidDocument()
  if (result.history !== undefined) {
    if (!Array.isArray(result.history)) return invalidDocument()
    for (const event of result.history) {
      if (!object(event) || typeof event.id !== 'string' || !uuid.test(event.id) || typeof event.action !== 'string' || !nullableString(event.reason_code) || !date(event.created_at) || (event.actor_id !== null && (typeof event.actor_id !== 'string' || !uuid.test(event.actor_id)))) return invalidDocument()
    }
  }
  return { ...result, ...summary } as unknown as SchoolChangeRequest
}
export function readDocument(document: ResourceDocument): SchoolChangeDocument & { size_bytes: number } {
  if (!document.data || Array.isArray(document.data)) return invalidDocument()
  const result = resource(document.data, 'miniapp-school-change-documents')
  if (result.mime !== 'image/png' || result.available !== true || !integer(result.size_bytes) || Number(result.size_bytes) < 1) return invalidDocument()
  return result as unknown as SchoolChangeDocument & { size_bytes: number }
}
export function readPolicy(document: ResourceDocument): SchoolChangePolicy {
  if (!document.data || Array.isArray(document.data) || document.data.type !== 'miniapp-school-change-policies' || document.data.id !== 'current') return invalidDocument()
  const result = decodeResource(document.data)
  if (result.enabled !== true || typeof result.student_number_required !== 'boolean' || typeof result.version !== 'string' || typeof result.notice !== 'string') return invalidDocument()
  for (const key of ['max_file_bytes', 'max_total_bytes', 'max_documents']) if (!integer(result[key])) return invalidDocument()
  if (!integer(result.review_working_days) || Number(result.review_working_days) < 1) return invalidDocument()
  if (!result.version.trim() || !result.notice.trim() || Number(result.max_documents) < 1 || Number(result.max_file_bytes) < 1 || Number(result.max_total_bytes) < Number(result.max_file_bytes)) return invalidDocument()
  return result as unknown as SchoolChangePolicy
}
