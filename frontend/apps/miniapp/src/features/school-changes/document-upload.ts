import type { SchoolChangeDocument } from './api'

type UploadedDocument = SchoolChangeDocument & { size_bytes: number }

export function acceptDocumentUpload(
  documents: UploadedDocument[],
  document: UploadedDocument,
  maxTotalBytes: number,
) {
  if (documents.reduce((sum, item) => sum + item.size_bytes, document.size_bytes) > maxTotalBytes)
    throw new Error('材料总大小超过限制')
  return { documents: [...documents, document], message: '证明材料已上传，提交申请后生效' }
}
