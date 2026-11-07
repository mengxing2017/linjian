import type {Client} from '../../http/client'
export type ImageRef = {kind:'asset';id:string}
export type ResourceAction = {kind:'none'}|{kind:'internal_page';page:'feed'|'resources'|'legal'|'post';object_id?:string|null}|{kind:'external_link';url:string}|{kind:'phone';number:string}|{kind:'copy_text';text:string}
export interface ResourceInput {id?:string|null;revision:number;kind:'banner'|'qrcode'|'service'|'legal'|'tag'|'blocked_word';title:string;content:string;image_ref:ImageRef|null;action:ResourceAction;active:boolean;sort_order:number;starts_at:string|null;ends_at:string|null}
export interface PublishedResource extends ResourceInput {id:string;image_url:string|null}
export interface Artwork {id:string;image_ref:ImageRef;url?:string;width?:number;height?:number}
export interface Confirmation {preview_id:string;digest:string}
export interface BusinessPreview<T> {id:string;digest:string;expires_at:string;summary:T;rendered?:string;sample?:boolean;requires_approval?:boolean}
export function createProductClient(client:Pick<Client,'requestResource'|'requestCollection'>){
 const {requestResource:r}=client
 const root=(s?:string)=>s?`/api/v1/schools/${encodeURIComponent(s)}`:'/api/v1/platform'
 return {
 resources:{
 list:(school?:string)=>r<PublishedResource[]>(`${root(school)}/${school?'published-resources':'resources'}`),
 preview:(school:string|undefined,input:ResourceInput)=>r<BusinessPreview<ResourceInput>>(`${root(school)}/resource-previews`,'POST',{...input,id:input.id??undefined}),
 confirm:(school:string|undefined,input:Confirmation)=>r<{id:string;published:boolean}>(`${root(school)}/resource-publications`,'POST',input),
 upload:(school:string|undefined,bytes:ArrayBuffer)=>r<Artwork>(`${root(school)}/resource-artwork`,'POST',bytes),
 artwork:(school?:string)=>r<Artwork[]>(`${root(school)}/resource-artwork`),
 qr:(school:string|undefined,url:string)=>r<Artwork>(`${root(school)}/resource-qrcodes`,'POST',{url}),
 },
 }
}
