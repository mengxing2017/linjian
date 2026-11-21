import { requestBinary, requestCollection, requestResource } from '../../http/browser-client'
import { uploadKey } from '../media/upload-key'
import { createSchoolsClient } from '../schools/client'
import type { SourceOutboxView } from '../source-access/types'
import { part, scope } from './paths'
import { postManagement } from './post-management'
import { siteSettings } from './site-settings'
import type {
  CaseManagementView,
  Comment,
  Post,
  PostReview,
  ResolveCaseInput,
  Resource,
  Row,
  School,
  SetCommentVisibility,
} from './types'
const schoolClient = createSchoolsClient({ requestResource, requestCollection, requestBinary })
export const workspace = {
  postManagement,
  siteSettings,
  terminateSource: (school: string, event: string, reason: string) =>
    requestResource<null>(scope(school, `source-outbox/${part(event)}/terminations`), 'POST', {
      reason,
    }),
  globalResources: () => requestResource<Row[]>('/api/v1/platform/resources'),
  schools: () => requestResource<School[]>('/api/v1/schools'),
  directory: schoolClient.directory,
  schoolProfiles: schoolClient,
  createSchool: (name: string) => requestResource<School>('/api/v1/schools', 'POST', { name }),
  updateSchool: (s: string, body: unknown) =>
    requestResource<School>(`/api/v1/schools/${part(s)}`, 'PUT', body),
  bind: (s: string) => requestResource(scope(s, 'bind'), 'POST'),
  list: <T extends Row = Row>(s: string, resource: Resource, query = '', signal?: AbortSignal) =>
    requestCollection<T>(scope(s, resource) + query, signal),
  outboxDetail: (s: string, id: string, signal?: AbortSignal) =>
    requestResource<SourceOutboxView>(
      scope(s, `source-outbox/${part(id)}`),
      'GET',
      undefined,
      signal,
    ),
  replaySource: (s: string, id: string, reason: string) =>
    requestResource(scope(s, `source-outbox/${part(id)}/replays`), 'POST', { reason }),
  cases: (s: string, query = '', signal?: AbortSignal) =>
    requestCollection<CaseManagementView>(scope(s, 'cases') + query, signal),
  resolveCase: (s: string, id: string, input: ResolveCaseInput) =>
    requestResource<CaseManagementView>(scope(s, `cases/${part(id)}`), 'PUT', input),
  reviews: (s: string, query = '', signal?: AbortSignal) =>
    requestCollection<PostReview>(scope(s, 'post-reviews') + query, signal),
  review: (s: string, id: string, signal?: AbortSignal) =>
    requestResource<PostReview>(scope(s, `posts/${part(id)}/review`), 'GET', undefined, signal),
  reviewImage: (s: string, id: string, signal?: AbortSignal) =>
    requestBinary(scope(s, `media/${part(id)}`), signal),
  retryPreparation: (s: string, id: string, reason: string, revision: number) =>
    requestResource(scope(s, `source-outbox/${part(id)}/preparation-retries`), 'POST', {
      reason,
      revision,
    }),
  getPost: (s: string, id: string) => requestResource<Post>(scope(s, `posts/${part(id)}`)),
  setCommentVisibility: (s: string, id: string, input: SetCommentVisibility) =>
    requestResource<Comment>(scope(s, `comments/${part(id)}/visibility`), 'PUT', input),
  comments: (s: string, id: string, page = 1) =>
    requestCollection<Row>(
      scope(s, `posts/${part(id)}/comments`) + `?page[number]=${page}&page[size]=30`,
    ),
  mutate: <T = Row>(s: string, path: string, body?: unknown, method = 'POST') =>
    requestResource<T>(scope(s, path), method, body),
  upload: (s: string, file: Blob) =>
    requestResource<{
      url: string
    }>(
      scope(s, 'media') + `?purpose=post_image&key=${encodeURIComponent(uploadKey())}`,
      'POST',
      file,
    ),
}
