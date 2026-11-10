import type { Client } from '../../http/client'
import { schoolPath } from '../schools'
export type MenuStatus = 'shown' | 'hot' | 'hidden'
export type EnableStatus = 'enabled' | 'pending' | 'disabled'
export interface HomeMenuTag { id: string; name: string; menu_status: MenuStatus; source_type: 'admin' | 'user'; enable_status: EnableStatus; sort_order: number; revision: number; created_at: string }
export interface HomeMenuSettings { latest_label: string; latest_visible: boolean; revision: number; catalog_revision: number }
export interface HomeMenu { id: string; settings: HomeMenuSettings; tags: HomeMenuTag[] }
export interface TagInput { name: string; menu_status: MenuStatus; enable_status: EnableStatus; revision?: number }
export function createHomeMenuClient(client: Client) {
 const path = (s: string, suffix: string) => `${schoolPath(s)}/${suffix}`
 return {
  search: (s: string, q: HomeMenuQuery) => client.requestResource<HomeMenuSearch>(path(s, 'home-menu-tags/search') + '?' + homeMenuQuery(q)),
  export: (s: string, q: HomeMenuQuery) => client.requestBinary(path(s, 'home-menu-tags/export') + '?' + homeMenuQuery(q)),
  commit: (s: string, id: string, digest: string) => client.requestResource<HomeMenuImport>(path(s, `home-menu-imports/${encodeURIComponent(id)}/commit`), 'POST', {digest}),
  public: (s: string) => client.requestResource<HomeMenu>(path(s, 'home-menu')),
  list: (s: string) => client.requestResource<HomeMenu>(path(s, 'home-menu-tags')),
  save: (s: string, input: TagInput, id?: string) => client.requestResource<HomeMenuTag>(path(s, id ? `home-menu-tags/${encodeURIComponent(id)}` : 'home-menu-tags'), id ? 'PUT' : 'POST', input),
  remove: (s: string, id: string, revision: number) => client.requestResource<null>(path(s, `home-menu-tags/${encodeURIComponent(id)}`), 'DELETE', {revision}),
  order: (s: string, ids: string[], catalog_revision: number) => client.requestResource<null>(path(s, 'home-menu-tags/order'), 'POST', {ids, catalog_revision}),
  getSettings: (s: string) => client.requestResource<HomeMenuSettings>(path(s, 'home-menu-settings')),
  settings: (s: string, settings: Pick<HomeMenuSettings, 'latest_label' | 'latest_visible' | 'revision'>) => client.requestResource<HomeMenuSettings>(path(s, 'home-menu-settings'), 'PUT', settings),
 }
}
export interface HomeMenuQuery { search?: string; menu_status?: MenuStatus; source_type?: 'admin' | 'user'; enable_status?: EnableStatus; page?: number }
export interface HomeMenuSearch { id: string; tags: HomeMenuTag[]; page: { number:number; size:number; has_more:boolean } }
export interface HomeMenuImport { id:string; digest:string; rows:Array<{row:number;name:string;menu_status:MenuStatus;enable_status:EnableStatus;sort_order:number}>; errors:Array<{row:number;message:string}>; expires_at:string; result:string[] | null }
export function homeMenuQuery(q: HomeMenuQuery): string {
 const p = new URLSearchParams()
 if(q.search) p.set('search',q.search)
 if(q.menu_status) p.set('menu_status',q.menu_status)
 if(q.source_type) p.set('source_type',q.source_type)
 if(q.enable_status) p.set('enable_status',q.enable_status)
 if(q.page) p.set('page[number]',String(q.page))
 return p.toString()
}
