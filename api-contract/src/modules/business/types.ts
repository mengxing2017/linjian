export type Json = string | number | boolean | null | Json[] | {
    [key: string]: Json;
};
export interface Row {
    id: string;
    [key: string]: Json;
}
export interface Grant {
    school_id: string;
    role: 'member' | 'operator' | 'admin';
}
export interface User {
    id: string;
    username: string;
    display_name: string;
    platform_admin: boolean;
    bound_school_id: string | null;
    grants: Grant[];
    wecom_authenticated: boolean;
}
export interface Session {
    user: User;
    csrf: string;
}
export type AuditMode = 'manual' | 'auto';
export type DecisionSource = 'human' | 'automatic' | 'legacy';
export interface School extends Row {
    name: string;
    status: 'normal' | 'hidden';
    short_name: string; province: string; city: string; area: string;
    latitude: number | null; longitude: number | null;
    logo_asset_id: string | null; header_bg_asset_id: string | null;
    logo_url: string | null; header_bg_image_url: string | null;
    theme_primary: string; theme_secondary: string; theme_text_color: string;
    revision: number;
    config: { [key: string]: Json } & { post_audit_mode?: AuditMode; comment_audit_mode?: AuditMode };
}
export interface Post extends Row {
    school_id: string;
    author_id: string;
    title: string;
    content: string;
    images: string[];
    tags: string[];
    audit_status: 'pending' | 'approved' | 'rejected';
    visibility: 'visible' | 'hidden' | 'deleted';
    revision: number;
}
export type Resource = 'source-outbox' | 'posts' | 'comments' | 'cases' | 'resources' | 'audit' | 'members' | 'follows' | 'notifications' | 'reactions';

export interface PostReview extends Row {
  school_id: string; school_name: string; author_id: string; author_nickname: string;
  title: string; content: string; images: string[]; tags: string[]; created_at: string;
  audit_status: 'pending' | 'approved' | 'rejected'; visibility: 'visible' | 'hidden' | 'deleted';
  top_sort: number; revision: number; content_revision: number; review_digest: string;
  decision_source: DecisionSource | null; decided_at: string | null; moderator_id: string | null;
}

export type CaseState = 'open' | 'resolved' | 'dismissed'
export type CaseKind = 'report' | 'feedback'
export interface CaseRecord extends Row {
  school_id: string
  account_id: string
  kind: CaseKind
  post_id: string | null
  content: string
  contact: string
  images: string[]
  state: CaseState
  resolution: string
  revision: number
  created_at: string
}
export interface CaseManagementView extends CaseRecord {
  title: string
  reporter_name: string
  school_name: string
  handled_at: string | null
  handled_by: string | null
}
export interface ResolveCaseInput {
  revision: number
  state: Exclude<CaseState, 'open'>
  resolution: string
}
