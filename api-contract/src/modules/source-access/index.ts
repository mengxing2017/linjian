import {requestResource} from '../../http'
export interface SourceConnection {id:string;name:string;origin:string;key_id:string;revision:number;checked_at:string;expires_at:string}
export interface SourceConnectionSettings {configured:boolean;origin:string|null;key_id:string|null;connection:SourceConnection|null}
export interface SourceBinding {id:string;route_id:string;active:boolean;revision:number}
export interface SchoolRouteSettings {binding:SourceBinding|null}
export const sourceAccess={
 connections:()=>requestResource<SourceConnectionSettings>('/api/v2/source-connections'),
 verify:()=>requestResource<{id:string;revision:number}>('/api/v2/source-connections','POST',{}),
 bindings:(school:string)=>requestResource<SchoolRouteSettings>(`/api/v2/schools/${encodeURIComponent(school)}/source-bindings`),
 bind:(school:string,v:{route_id:string;active:boolean})=>requestResource<{id:string}>(`/api/v2/schools/${encodeURIComponent(school)}/source-bindings`,'POST',v),
}

export type {SourceEventV2,SourceReceiptV2,SourceCheckV2,SourceCapabilitiesV2,SourceDocument} from './protocol'
