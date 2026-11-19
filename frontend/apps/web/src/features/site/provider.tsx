import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { siteSettings as api, type PublicSiteSettings } from './api'
import { EMPTY_SITE_SETTINGS } from './projection'
import { entryTitle, pageTitle } from './title'

const STORAGE_KEY = 'linjian.site-settings.v1'
const QUERY_KEY = ['site-settings', 'public'] as const

interface SiteSettingsContextValue {
  settings: PublicSiteSettings
  loading: boolean
  stale: boolean
  refresh: () => void
  apply: (settings: PublicSiteSettings) => void
}

const SiteSettingsContext = createContext<SiteSettingsContextValue>({
  settings: EMPTY_SITE_SETTINGS,
  loading: false,
  stale: false,
  refresh: () => {},
  apply: () => {},
})

function readCached(): PublicSiteSettings | undefined {
  if (typeof localStorage === 'undefined') return undefined
  try {
    const value = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null') as unknown
    if (
      value &&
      typeof value === 'object' &&
      'revision' in value &&
      typeof value.revision === 'number' &&
      value.revision >= 0
    )
      return value as PublicSiteSettings
  } catch {
    return undefined
  }
  return undefined
}

function writeCached(value: PublicSiteSettings) {
  if (typeof localStorage === 'undefined') return
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
  } catch {
    // A full or unavailable local store must not block login or content.
  }
}

export function SiteSettingsProvider({ children }: { children: ReactNode }) {
  const cache = useQueryClient()
  const [cached, setCached] = useState<PublicSiteSettings | undefined>(readCached)
  const query = useQuery({
    queryKey: QUERY_KEY,
    queryFn: ({ signal }) => api.public(signal),
    staleTime: 60_000,
    refetchInterval: 5 * 60_000,
    refetchOnWindowFocus: true,
    retry: 1,
  })
  useEffect(() => {
    const data = query.data
    if (!data || cached === data) return
    if (cached && data.revision < cached.revision) return
    setCached(data)
    writeCached(data)
  }, [cached, query.data])
  const apply = useCallback(
    (value: PublicSiteSettings) => {
      setCached(value)
      writeCached(value)
      cache.setQueryData(QUERY_KEY, value)
    },
    [cache],
  )
  const settings = cached ?? query.data ?? EMPTY_SITE_SETTINGS
  const value = useMemo(
    () => ({
      settings,
      loading: query.isPending && !cached,
      stale: query.isError && !!cached,
      refresh: () => void query.refetch(),
      apply,
    }),
    [apply, cached, query, settings],
  )
  return (
    <SiteSettingsContext.Provider value={value}>
      {children}
    </SiteSettingsContext.Provider>
  )
}

export function useSiteSettings(): SiteSettingsContextValue {
  return useContext(SiteSettingsContext)
}

/** Runtime title only; the first HTML response deliberately carries no brand. */
export function useSiteTitle(page?: string) {
  const { settings } = useSiteSettings()
  const title =
    page === undefined ? entryTitle(settings) : pageTitle(page, settings)
  useEffect(() => {
    document.title = title
  }, [title])
}
