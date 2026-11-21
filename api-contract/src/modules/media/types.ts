export type UploadPurpose = 'post_image' | 'feedback_evidence'
export interface UploadedMedia {
  id: string
  purpose: string
  mime_type: string
  size_bytes: number
  width: number
  height: number
  expires_at: string
  url: string
}
