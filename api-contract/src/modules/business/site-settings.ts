import { requestResource } from "../../http";

export type CopyrightEndMode = "auto" | "fixed";

export interface PublicSiteSettings {
  revision: number;
  site_name: string | null;
  title_subtitle: string | null;
  login_intro: string | null;
  icp_number: string | null;
  police_number: string | null;
  police_url: string | null;
  copyright_owner: string | null;
  copyright_years: string | null;
  copyright_notice: string | null;
}

export interface SiteSettingsHistory {
  id: string;
  actor_name: string | null;
  created_at: string;
  revision: number;
  changed: string[];
  cleared: string[];
  previous: Record<string, unknown>;
  current: Record<string, unknown>;
}

export interface AdminSiteSettings extends PublicSiteSettings {
  copyright_start_year: number | null;
  copyright_end_mode: CopyrightEndMode | null;
  copyright_end_year: number | null;
  updated_at: string;
  updated_by_name: string | null;
  history: SiteSettingsHistory[];
}

export interface SiteSettingsInput {
  revision: number;
  site_name: string | null;
  title_subtitle: string | null;
  login_intro: string | null;
  icp_number: string | null;
  police_number: string | null;
  police_url: string | null;
  copyright_owner: string | null;
  copyright_start_year: number | null;
  copyright_end_mode: CopyrightEndMode | null;
  copyright_end_year: number | null;
  copyright_notice: string | null;
}

export const siteSettings = {
  public: (signal?: AbortSignal) =>
    requestResource<PublicSiteSettings>(
      "/api/v1/site-settings",
      "GET",
      undefined,
      signal,
    ),
  read: (signal?: AbortSignal) =>
    requestResource<AdminSiteSettings>(
      "/api/v1/platform/site-settings",
      "GET",
      undefined,
      signal,
    ),
  update: (input: SiteSettingsInput) =>
    requestResource<AdminSiteSettings>(
      "/api/v1/platform/site-settings",
      "PUT",
      input,
    ),
};
