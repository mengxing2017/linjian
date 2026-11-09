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
export interface School extends Row {
    name: string;
    status: 'normal' | 'hidden';
    short_name: string; province: string; city: string; area: string;
    latitude: number | null; longitude: number | null;
    logo_asset_id: string | null; header_bg_asset_id: string | null;
    logo_url: string | null; header_bg_image_url: string | null;
    theme_primary: string; theme_secondary: string; theme_text_color: string;
    revision: number;
    config: {
        [key: string]: Json;
    };
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
