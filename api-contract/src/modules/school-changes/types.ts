export type SchoolChangeStatus = 'pending' | 'approved' | 'rejected' | 'cancelled' | 'invalidated'
export interface SchoolChangeDocument {
  id: string
  mime: string
  available: boolean
}
export interface SchoolChangeSummary {
  id: string
  account_id: string
  from_school_id: string
  from_school_name: string
  to_school_id: string
  to_school_name: string
  status: SchoolChangeStatus
  revision: number
  submitted_at: string
  review_due_at: string
  closed_at: string | null
  reviewed_at: string | null
  reviewed_by: string | null
  consent_version: string
  result_reason_code: string | null
  personal_data_purged: boolean
  blocked_reasons: string[]
}
export interface SchoolChangeRequest extends SchoolChangeSummary {
  student_relationship: 'enrolled' | 'new_student' | null
  declared_student_name: string | null
  declared_student_number: string | null
  reason: string | null
  result_reason: string | null
  documents: SchoolChangeDocument[]
  review_guidance?: string
  history?: Array<{ id: string; action: string; actor_id: string | null; reason_code: string | null; created_at: string }>
}
export interface SchoolChangePolicy {
  id: string
  enabled: true
  version: string
  notice: string
  student_number_required: boolean
  max_file_bytes: number
  max_total_bytes: number
  max_documents: number
  review_working_days: number
}
export interface SchoolChangeSubmission {
  to_school_id: string
  student_relationship: 'enrolled' | 'new_student'
  declared_student_name: string
  declared_student_number: string | null
  reason: string
  document_ids: string[]
  consent_version: string
  submission_key: string
}
export interface SchoolChangeFilters {
  status?: SchoolChangeStatus
  account_id?: string
  page?: number
  created_from?: string
  created_to?: string
}
