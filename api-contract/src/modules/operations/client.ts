import type { Cancellation, Client } from '../../http/client'
import { operationsQuery } from './query'
import type {
  CustomerConfig,
  CustomerInput,
  Display,
  Feedback,
  FeedbackInput,
  Filters,
  Module,
  OperationInput,
  OperationResource,
} from './types'

export function createOperationsClient(c: Client) {
  const root = (s?: string) => (s ? `/api/v1/schools/${encodeURIComponent(s)}` : '/api/v1/platform')
  return {
    postOptions: (
      school: string,
      context: 'operation.banner.create' | 'operation.banner.update' | 'resource.school.write',
      search = '',
      signal?: Cancellation,
    ) =>
      c.requestCollection<{ id: string; title: string }>(
        `${root(school)}/operation-post-options?context=${encodeURIComponent(context)}&search=${encodeURIComponent(search)}`,
        signal,
      ),
    list: (s: string | undefined, m: Module, q: Filters = {}, signal?: Cancellation) =>
      c.requestCollection<OperationResource>(`${root(s)}/${m}${operationsQuery(q)}`, signal),
    get: (s: string | undefined, m: Module, id: string) =>
      c.requestResource<OperationResource>(`${root(s)}/${m}/${encodeURIComponent(id)}`),
    save: (s: string | undefined, m: Module, v: OperationInput, id?: string) =>
      c.requestResource<OperationResource>(
        `${root(s)}/${m}${id ? `/${encodeURIComponent(id)}` : ''}`,
        id ? 'PATCH' : 'POST',
        v,
      ),
    remove: (s: string | undefined, m: Module, id: string, revision: number) =>
      c.requestResource<null>(`${root(s)}/${m}/${encodeURIComponent(id)}`, 'DELETE', { revision }),
    customer: () => c.requestResource<CustomerConfig>('/api/v1/platform/customer-config'),
    saveCustomer: (v: CustomerInput) =>
      c.requestResource<CustomerConfig>('/api/v1/platform/customer-config', 'PATCH', v),
    feedback: (s: string, q: Filters = {}, signal?: Cancellation) =>
      c.requestCollection<Feedback>(`${root(s)}/feedback${operationsQuery(q)}`, signal),
    feedbackDetail: (s: string, id: string) =>
      c.requestResource<Feedback>(`${root(s)}/feedback/${encodeURIComponent(id)}`),
    deleteFeedback: (s: string, id: string, revision: number) =>
      c.requestResource<null>(`${root(s)}/feedback/${encodeURIComponent(id)}`, 'DELETE', {
        revision,
      }),
    submit: (s: string, v: FeedbackInput) =>
      c.requestResource<{ id: string; submitted: boolean }>(`${root(s)}/feedback`, 'POST', v),
    display: (s: string) => c.requestResource<Display>(`${root(s)}/operations-display`),
    event: (
      s: string,
      v: {
        event_id: string
        resource_id: string
        event_type: 'view' | 'click'
        display_cycle: string
        occurred_at: string
      },
    ) =>
      c.requestResource<{ id: string; accepted: boolean }>(`${root(s)}/operation-events`, 'POST', {
        ...v,
        channel: null,
        target_version_id: null,
        placement: 'home_banner',
        attempt_id: v.event_type === 'click' ? v.event_id : null,
      }),
  }
}
