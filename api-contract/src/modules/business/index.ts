import { requestResource, requestCollection, requestBinary, setCsrf } from '../../http';
export { ApiError } from '../../http';
export * from './types'
export * from './post-management'
import {createSchoolsClient} from '../schools'
import type {SourceOutboxView} from '../source-access'
import type { Comment, SetCommentVisibility, Row, Session, School, Post, PostReview, Resource, CaseManagementView, ResolveCaseInput } from './types'
const schoolClient=createSchoolsClient({requestResource,requestCollection,requestBinary})
const part = (s: string) => encodeURIComponent(s);
export const scope = (school: string, path: string) => `/api/v1/schools/${part(school)}/${path}`;
export const identity = {
    async session() { const v = await requestResource<Session>('/api/v1/auth/session'); setCsrf(v.csrf); return v; },
    async login(username: string, password: string) { const v = await requestResource<Session>('/api/v1/auth/sessions', 'POST', { username, password }); setCsrf(v.csrf); return v; },
    async logout() { await requestResource('/api/v1/auth/session', 'DELETE'); setCsrf(''); },
    profile: (display_name: string) => requestResource('/api/v1/profile', 'PUT', { display_name }),
    accounts: () => requestResource<Row[]>('/api/v1/accounts'),
    create: (body: {
        username: string;
        display_name: string;
        password: string;
    }) => requestResource<Row>('/api/v1/accounts', 'POST', body),
    update: (id: string, body: {display_name: string; active: boolean}) => requestResource(`/api/v1/accounts/${part(id)}`, 'PUT', body),
    reset: (id: string, password: string) => requestResource(`/api/v1/accounts/${part(id)}/password`, 'POST', { username: '', password }),
};
export interface WecomStatus { server_configured: boolean; client_configured: boolean; oauth_available: boolean; share_available: boolean }
import { postManagement } from './post-management'
export const workspace = {
    postManagement,
    wecomStatus: () => requestResource<WecomStatus>('/api/v1/wecom/status'),
    globalResources: () => requestResource<Row[]>('/api/v1/platform/resources'),
    schools: () => requestResource<School[]>('/api/v1/schools'),
    directory: schoolClient.directory,
    schoolProfiles: schoolClient,
    createSchool: (name: string) => requestResource<School>('/api/v1/schools', 'POST', { name }),
    updateSchool: (s: string, body: unknown) => requestResource<School>(`/api/v1/schools/${part(s)}`, 'PUT', body),
    bind: (s: string) => requestResource(scope(s, 'bind'), 'POST'),
    list: <T extends Row = Row>(s: string, resource: Resource, query = '', signal?: AbortSignal) => requestCollection<T>(scope(s, resource) + query, signal),
    outboxDetail: (s:string,id:string,signal?:AbortSignal) => requestResource<SourceOutboxView>(scope(s,`source-outbox/${part(id)}`),'GET',undefined,signal),
    replaySource: (s:string,id:string,reason:string) => requestResource(scope(s,`source-outbox/${part(id)}/replays`),'POST',{reason}),
    cases: (s:string,query='',signal?:AbortSignal)=>requestCollection<CaseManagementView>(scope(s,'cases')+query,signal),
    resolveCase: (s:string,id:string,input:ResolveCaseInput)=>requestResource<CaseManagementView>(scope(s,`cases/${part(id)}`),'PUT',input),
    reviews: (s:string,query='',signal?:AbortSignal)=>requestCollection<PostReview>(scope(s,'post-reviews')+query,signal),
    review: (s:string,id:string,signal?:AbortSignal)=>requestResource<PostReview>(scope(s,`posts/${part(id)}/review`),'GET',undefined,signal),
    reviewImage: (s:string,id:string,signal?:AbortSignal)=>requestBinary(scope(s,`media/${part(id)}`),signal),
    retryPreparation: (s:string,id:string,reason:string,revision:number)=>requestResource(scope(s,`source-outbox/${part(id)}/preparation-retries`),'POST',{reason,revision}),
    getPost: (s: string, id: string) => requestResource<Post>(scope(s, `posts/${part(id)}`)),
    setCommentVisibility: (s: string, id: string, input: SetCommentVisibility) => requestResource<Comment>(scope(s, `comments/${part(id)}/visibility`), 'PUT', input),
    comments: (s: string, id: string, page = 1) => requestCollection<Row>(scope(s, `posts/${part(id)}/comments`) + `?page[number]=${page}&page[size]=30`),
    mutate: <T = Row>(s: string, path: string, body?: unknown, method = 'POST') => requestResource<T>(scope(s, path), method, body),
    upload: (s: string, file: Blob) => requestResource<{
        url: string;
    }>(scope(s, 'media'), 'POST', file),

};

export interface DirectoryDepartment { id: number; parentid: number; name: string; order: number }
export interface DirectoryMember {
    id: string; userid: string; name: string; status: number | null;
    visibility: 'visible' | 'not_seen'; verification: 'verified' | 'unavailable' | 'unverified';
    departments: DirectoryDepartment[]; checked_at: string | null;
    bound_account_id: string | null; bound_account_name: string | null;
}
export interface DirectoryRun {
    id: string; state: 'queued' | 'running' | 'succeeded' | 'failed';
    created_at: string; started_at: string | null; finished_at: string | null;
    error_code: string | null; department_count: number; member_count: number;
}
export interface DirectoryStatus {
    id: string; server_configured: boolean; configuration_error: string | null;
    credential_generation: number | null; credential_rotation_required: boolean;
    snapshot_id: string | null; last_success_at: string | null; latest_run: DirectoryRun | null;
}
export interface DirectoryFilter { page?: number; search?: string; department_id?: number; status?: number; binding?: 'bound' | 'unbound' }
const directoryPath = '/api/v1/platform/wecom-directory';
export const directory = {
    status: () => requestResource<DirectoryStatus>(`${directoryPath}/status`),
    members: (filter: DirectoryFilter = {}) => {
        const q = new URLSearchParams({'page[number]': String(filter.page ?? 1), 'page[size]': '30'});
        for (const key of ['search','department_id','status','binding'] as const) if (filter[key] !== undefined && filter[key] !== '') q.set(key, String(filter[key]));
        return requestCollection<DirectoryMember>(`${directoryPath}/members?${q}`);
    },
    async departments() {
        const items: DirectoryDepartment[] = [];
        for (let page = 1; ; page++) {
            const result = await requestCollection<Omit<DirectoryDepartment,'id'> & {id:string}>(`${directoryPath}/departments?page[number]=${page}&page[size]=100`);
            items.push(...result.items.map(d => ({...d,id:Number(d.id)})));
            if (!result.page?.has_more) return items;
        }
    },
    rotate: (expected_generation: number) => requestResource(`${directoryPath}/credential-rotations`, 'POST', {expected_generation}),
    sync: () => requestResource<DirectoryRun>(`${directoryPath}/sync-runs`, 'POST'),
    run: (id: string) => requestResource<DirectoryRun>(`${directoryPath}/sync-runs/${part(id)}`),
    bind: (account: string, member_id: string, revision: number) => requestResource(`/api/v1/accounts/${part(account)}/wecom-binding`, 'PUT', {member_id, revision}),
    unbind: (account: string, revision: number) => requestResource(`/api/v1/accounts/${part(account)}/wecom-binding`, 'DELETE', undefined, undefined, {'If-Match': `"${revision}"`}),
};

export * from '../product-workflows'

export function wecomOAuthUrl(selection: {school?:string} = {}) {
 const params = new URLSearchParams(); if(selection.school)params.set('school',selection.school);
 return `/api/v1/wecom/oauth/start${params.size?`?${params}`:''}`;
}

export interface BlockedWord { id: string; school_id: string | null; word: string; remark: string; active: boolean; revision: number; sort_order: number }
export interface BlockedWordCatalog { id: string; items: BlockedWord[]; order_revision: number; has_more:boolean }
export interface BlockedWordInput { word: string; remark: string; active: boolean; revision?: number }
export interface BlockedWordImport { id:string; digest:string; rows:Array<{row:number;word:string;remark:string;active:boolean}>; errors:Array<{row:number;message:string}>; expires_at:string }
export function createBlockedWordsClient(client: import("../../http/client").Client) {
 const path=(school?: string)=>school?`/api/v1/schools/${encodeURIComponent(school)}/blocked-words`:'/api/v1/platform/blocked-words'
 return {
  list:(school:string|undefined, global:boolean, search:string, status:string, page:number, ordering:boolean)=>client.requestResource<BlockedWordCatalog>(path(school)+'?'+new URLSearchParams({global:String(global),search,status:status==='all'?'':status,page:String(page),ordering:String(ordering)})),
  save:(school:string|undefined, input:BlockedWordInput, id?:string)=>client.requestResource<BlockedWord>(path(school)+(id?`/${encodeURIComponent(id)}`:''),id?'PUT':'POST',input),
  remove:(school:string|undefined,id:string,revision:number)=>client.requestResource<null>(`${path(school)}/${encodeURIComponent(id)}`,'DELETE',{revision}),
  order:(school:string|undefined,ids:string[],order_revision:number)=>client.requestResource<null>(`${path(school)}/order`,'POST',{ids,order_revision}),
  commitImport:(school:string|undefined,id:string,digest:string)=>client.requestResource<{id:string;count:number}>(`${school?`/api/v1/schools/${encodeURIComponent(school)}`:'/api/v1/platform'}/blocked-word-imports/${encodeURIComponent(id)}/commit`,'POST',{digest}),
 }
}
