export type AdvertisementStatus = 'draft' | 'pending' | 'approved' | 'rejected' | 'withdrawn'
export type AdvertisementUsageResult = 'applied' | 'no_approved_advertisement' | 'message_budget_exceeded'

export interface AdvertisementDecision {
  id: string
  decision: 'approve' | 'reject'
  reason: string
  reviewer_id: string
  reviewed_at: string
}

export interface AdvertisementRevision {
  id: string
  advertisement_id: string
  school_id: string
  school_name: string
  number: string
  version: string
  config_version: string
  content: string
  status: AdvertisementStatus
  review_digest: string | null
  current_revision_id: string | null
  config_deleted_at: string | null
  created_by: string
  created_at: string
  submitted_by: string | null
  submitted_at: string | null
  decision: AdvertisementDecision | null
  configuration?: MessageAdvertisement
}

export interface AdvertisementUsage {
  event_id: string
  revision_id: string | null
  result: AdvertisementUsageResult
  base_length: number
  final_length: number
  frozen_at: string
}

export interface MessageAdvertisement {
  id: string
  school_id: string
  version: string
  created_at: string
  deleted_at: string | null
  deletion_reason: string | null
  current_revision: AdvertisementRevision | null
  candidate: AdvertisementRevision | null
  recent_usage: AdvertisementUsage[]
}

export interface AdvertisementFilters {
  school_id?: string
  status?: AdvertisementStatus
  page?: number
  created_from?: string
  created_to?: string
}

export const advertisementStatusLabels: Record<AdvertisementStatus, string> = {
  draft: '草稿', pending: '待平台审核', approved: '已通过', rejected: '已驳回', withdrawn: '已撤回',
}

export function normalizeAdvertisement(value: string): string {
  return value.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '')
}

export function advertisementContentError(value: string): string | null {
  const content = normalizeAdvertisement(value)
  const length = Array.from(content).length
  if (length < 1 || length > 50) return '广告文案须为 1–50 个 Unicode 字符'
  if (/[\p{Cc}\u2028\u2029\ufeff\ud800-\udfff]/u.test(content)) return '广告文案不能包含换行或控制字符'
  return null
}
