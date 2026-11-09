import type { Client, Cancellation } from "../../http/client";
import type { School } from "../business/types";
export type { School } from "../business/types";
export type SchoolDisplay = Pick<
  School,
  | "id"
  | "name"
  | "short_name"
  | "province"
  | "city"
  | "area"
  | "latitude"
  | "longitude"
  | "status"
  | "logo_url"
  | "header_bg_image_url"
  | "theme_primary"
  | "theme_secondary"
  | "theme_text_color"
  | "revision"
>;
export interface SchoolInput {
  name: string;
  short_name?: string;
  province?: string;
  city?: string;
  area?: string;
  latitude?: number | null;
  longitude?: number | null;
  status?: "normal" | "hidden";
  logo_asset_id?: string | null;
  header_bg_asset_id?: string | null;
  theme_primary?: string;
  theme_secondary?: string;
  theme_text_color?: string;
  revision?: number;
  config?: School["config"];
}
export interface SchoolFilters {
  page?: number;
  search?: string;
  province?: string;
  city?: string;
  area?: string;
  status?: "normal" | "hidden";
}
export const schoolPath = (school: string) =>
  `/api/v1/schools/${encodeURIComponent(school)}`;
export function createSchoolsClient(
  client: Pick<
    Client,
    "requestResource" | "requestCollection" | "requestBinary"
  >,
) {
  const query = (filters: SchoolFilters = {}) => {
    const { page, ...rest } = filters;
    const pairs = [
      ["page[number]", String(page ?? 1)],
      ["page[size]", "30"],
    ];
    for (const [k, v] of Object.entries(rest))
      if (v !== undefined && v !== "") pairs.push([k, String(v)]);
    return (
      "?" +
      pairs
        .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`)
        .join("&")
    );
  };
  return {
    list: () => client.requestResource<SchoolDisplay[]>("/api/v1/schools"),
    platform: (filters: SchoolFilters = {}, signal?: Cancellation) =>
      client.requestCollection<School>(
        "/api/v1/platform/schools" + query(filters),
        signal,
      ),
    directory: (filters: SchoolFilters = {}, signal?: Cancellation) =>
      client.requestCollection<SchoolDisplay>(
        "/api/v1/school-directory" + query(filters),
        signal,
      ),
    detail: (school: string, signal?: Cancellation) =>
      client.requestResource<School>(
        schoolPath(school),
        "GET",
        undefined,
        signal,
      ),
    create: (body: SchoolInput) =>
      client.requestResource<School>("/api/v1/schools", "POST", body),
    update: (school: string, body: SchoolInput) =>
      client.requestResource<School>(schoolPath(school), "PUT", body),
    bind: (school: string) =>
      client.requestResource<null>(`${schoolPath(school)}/bind`, "POST"),
    branding: (school: string, slot: "logo" | "header") =>
      client.requestBinary(
        `/api/v1/school-directory/${encodeURIComponent(school)}/branding/${slot}`,
      ),
    artwork: (school: string, id: string) =>
      client.requestBinary(
        `${schoolPath(school)}/resource-artwork/${encodeURIComponent(id)}`,
      ),
  };
}
