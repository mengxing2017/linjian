import { decodeResource, invalidDocument, object, type ResourceDocument } from '../../http/protocol'
import { advertisementContentError, normalizeAdvertisement, type AdvertisementRevision, type MessageAdvertisement } from './types'

const statuses = ['draft', 'pending', 'approved', 'rejected', 'withdrawn']
const string = (value: unknown): value is string => typeof value === 'string' && value.length > 0
const nullableString = (value: unknown) => value === null || string(value)
const version = (value: unknown) => typeof value === 'string' && /^[1-9]\d*$/.test(value)

export function readRevision(value: unknown): AdvertisementRevision {
  if (!object(value) || !['id', 'advertisement_id', 'school_id', 'school_name', 'created_by', 'created_at'].every(key => string(value[key]))
    || !version(value.version) || !version(value.config_version) || !version(value.number)
    || typeof value.status !== 'string' || !statuses.includes(value.status)
    || typeof value.content !== 'string' || advertisementContentError(value.content) || normalizeAdvertisement(value.content) !== value.content
    || !['review_digest', 'current_revision_id', 'config_deleted_at', 'submitted_by', 'submitted_at'].every(key => nullableString(value[key]))) return invalidDocument()
  if (value.review_digest !== null && !/^[0-9a-f]{64}$/.test(String(value.review_digest))) return invalidDocument()
  if (value.decision !== null && (!object(value.decision)
    || !['id', 'reviewer_id', 'reviewed_at'].every(key => string((value.decision as Record<string, unknown>)[key]))
    || !['approve', 'reject'].includes(String(value.decision.decision)) || typeof value.decision.reason !== 'string')) return invalidDocument()
  if (['pending', 'approved', 'rejected'].includes(value.status) && (value.review_digest === null || value.submitted_by === null || value.submitted_at === null)) return invalidDocument()
  if (value.status === 'approved' && (!object(value.decision) || value.decision.decision !== 'approve')) return invalidDocument()
  if (value.status === 'rejected' && (!object(value.decision) || value.decision.decision !== 'reject')) return invalidDocument()
  if (value.configuration !== undefined) readConfiguration(value.configuration)
  return value as unknown as AdvertisementRevision
}

export function readConfiguration(value: unknown): MessageAdvertisement {
  if (!object(value) || !['id', 'school_id', 'created_at'].every(key => string(value[key])) || !version(value.version)
    || !nullableString(value.deleted_at) || !(value.deletion_reason === null || typeof value.deletion_reason === 'string')
    || !Array.isArray(value.recent_usage)) return invalidDocument()
  if (value.current_revision !== null) {
    const current = readRevision(value.current_revision)
    if (current.advertisement_id !== value.id || current.school_id !== value.school_id || current.status !== 'approved') return invalidDocument()
  }
  if (value.candidate !== null) {
    const candidate = readRevision(value.candidate)
    if (candidate.advertisement_id !== value.id || candidate.school_id !== value.school_id || !['draft', 'pending'].includes(candidate.status)) return invalidDocument()
  }
  for (const usage of value.recent_usage) {
    if (!object(usage) || !string(usage.event_id) || !nullableString(usage.revision_id) || !string(usage.frozen_at)
      || !['applied', 'no_approved_advertisement', 'message_budget_exceeded'].includes(String(usage.result))
      || !Number.isInteger(usage.base_length) || !Number.isInteger(usage.final_length)
      || Number(usage.base_length) < 1 || Number(usage.final_length) < 1 || Number(usage.final_length) > 200) return invalidDocument()
  }
  return value as unknown as MessageAdvertisement
}

export function readSingle(document: ResourceDocument, kind: string): Record<string, unknown> | null {
  if (document.data === null) return null
  if (Array.isArray(document.data) || document.data.type !== kind) return invalidDocument()
  return decodeResource(document.data)
}
