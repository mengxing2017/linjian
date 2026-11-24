import type { Client } from '../../http/client'
import type { Session, OnboardingResult, OnboardingInput, OnboardingSchool } from './types'

export function createIdentityClient(client: Client) {
  return {
    startOnboarding: (code: string) => client.requestResource<OnboardingResult>('/api/v1/auth/miniapp/onboarding', 'POST', { code }),
    onboarding: (action: 'status' | 'confirm' | 'complete', input: OnboardingInput) => client.requestResource<OnboardingResult>(`/api/v1/auth/miniapp/onboarding/${action}`, 'POST', input),
    cancelOnboarding: (ticket: string) => client.requestResource<{ state: 'cancelled' }>('/api/v1/auth/miniapp/onboarding/cancel', 'POST', { ticket }),
    onboardingSchools: (query = '', page = 1) => client.requestResource<OnboardingSchool[]>(`/api/v1/onboarding/schools?query=${encodeURIComponent(query)}&page=${page}`),
    session: () => client.requestResource<Session>('/api/v1/auth/session'),
    logout: () => client.requestResource<null>('/api/v1/auth/session', 'DELETE'),
    profile: (display_name: string) =>
      client.requestResource<{ id: string; display_name: string }>('/api/v1/profile', 'PUT', {
        display_name,
      }),
  }
}
