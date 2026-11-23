use axum::extract::Query;
use linjian_api::{business::Page, community, miniapp_users, operations, school_changes, schools};
use serde::de::DeserializeOwned;

fn query<T: DeserializeOwned>(parameters: &str) -> T {
    let uri = format!("/api/v1/query-check?{parameters}").parse().unwrap();
    Query::<T>::try_from_uri(&uri)
        .unwrap_or_else(|error| panic!("{}: {error}", std::any::type_name::<T>()))
        .0
}

fn rejects<T: DeserializeOwned>(parameters: &str) {
    let uri = format!("/api/v1/query-check?{parameters}").parse().unwrap();
    assert!(
        Query::<T>::try_from_uri(&uri).is_err(),
        "{} accepted invalid parameters",
        std::any::type_name::<T>()
    );
}

#[test]
fn shared_page_supports_community_reviews_cases_outbox_and_audit_queries() {
    for parameters in [
        "page[number]=2&page[size]=30&search=test&status=pending&visibility=visible&mine=false&kind=report&tag=test",
        "page%5Bnumber%5D=2&page%5Bsize%5D=30&search=test&status=pending&visibility=visible&mine=false&kind=report&tag=test",
    ] {
        let page: Page = query(parameters);
        assert_eq!(page.bounds().unwrap(), (30, 30));
        assert_eq!(page.mine, Some(false));
        assert_eq!(page.status.as_deref(), Some("pending"));
        assert_eq!(page.visibility.as_deref(), Some("visible"));
        assert_eq!(page.kind.as_deref(), Some("report"));
        assert_eq!(page.tag.as_deref(), Some("test"));
    }
    assert_eq!(query::<Page>("").bounds().unwrap(), (30, 0));
}

#[test]
fn school_directory_and_platform_school_queries_accept_all_filters() {
    use schools::domain::profile::SchoolQuery;
    let filters: SchoolQuery = query(
        "page[number]=2&page[size]=10&search=test&province=test&city=test&area=test&status=normal",
    );
    assert_eq!(filters.bounds().unwrap(), (10, 10));
    assert_eq!(filters.search.as_deref(), Some("test"));
    assert_eq!(query::<SchoolQuery>("").bounds().unwrap(), (30, 0));
    rejects::<SchoolQuery>("page[number]=abc");
}

#[test]
fn member_and_candidate_queries_accept_boolean_filters() {
    use schools::domain::permissions::MemberQuery;
    for has_overrides in [true, false] {
        let filters: MemberQuery = query(&format!(
            "page[number]=2&page[size]=10&search=test&role=operator&has_overrides={has_overrides}"
        ));
        assert_eq!(filters.bounds().unwrap(), (11, 10));
        assert_eq!(filters.has_overrides, Some(has_overrides));
    }
    assert_eq!(query::<MemberQuery>("").bounds().unwrap(), (31, 0));
    rejects::<MemberQuery>("has_overrides=invalid");
}

#[test]
fn home_menu_queries_accept_enum_and_pagination_filters() {
    use schools::domain::home_menu::{CatalogQuery, EnableStatus, MenuStatus, SourceType};
    let filters: CatalogQuery = query(
        "page[number]=2&page[size]=10&search=test&menu_status=shown&source_type=admin&enable_status=enabled",
    );
    assert_eq!(filters.bounds().unwrap(), (10, 10));
    assert_eq!(filters.menu_status, Some(MenuStatus::Shown));
    assert_eq!(filters.source_type, Some(SourceType::Admin));
    assert_eq!(filters.enable_status, Some(EnableStatus::Enabled));
    assert_eq!(query::<CatalogQuery>("").bounds().unwrap(), (20, 0));
    rejects::<CatalogQuery>("menu_status=invalid");
}

#[test]
fn blocked_word_queries_keep_their_plain_page_parameter() {
    use schools::domain::blocked_words::WordQuery;
    for value in [true, false] {
        let filters: WordQuery = query(&format!(
            "page=2&global={value}&ordering={value}&search=test&status="
        ));
        assert_eq!(filters.page, Some(2));
        assert_eq!(filters.global, Some(value));
        assert_eq!(filters.ordering, Some(value));
    }
    assert_eq!(query::<WordQuery>("").page, None);
    rejects::<WordQuery>("ordering=invalid");
}

#[test]
fn post_management_queries_accept_boolean_uuid_and_offset_dates() {
    use community::domain::post_management::PostManagementFilters;
    for is_top in [true, false] {
        let filters: PostManagementFilters = query(&format!(
            "page[number]=2&page[size]=10&search=test&audit_status=approved&visibility=visible&is_top={is_top}&tag=test&created_via=seed&author_kind=simulated&seed_batch_id=01a201bc-0058-7006-9181-9e875955c7e4&display_from=2026-11-01T00%3A00%3A00%2B08%3A00&display_to=2026-11-23T00%3A00%3A00Z&created_from=2026-11-01T00%3A00%3A00Z&created_to=2026-11-23T00%3A00%3A00Z"
        ));
        filters.validate().unwrap();
        assert_eq!(filters.pagination().bounds().unwrap(), (10, 10));
        assert_eq!(filters.is_top, Some(is_top));
        assert!(filters.seed_batch_id.is_some());
        assert!(filters.display_from.is_some());
    }
    assert_eq!(query::<PostManagementFilters>("").page, None);
    rejects::<PostManagementFilters>("is_top=invalid");
    rejects::<PostManagementFilters>("seed_batch_id=invalid");
    rejects::<PostManagementFilters>("created_from=invalid");
}

#[test]
fn post_seed_batch_item_and_author_queries_accept_pagination() {
    use community::domain::post_seed::SeedQuery;
    let filters: SeedQuery = query("page[number]=2&page[size]=10&search=test&state=ready");
    assert_eq!(filters.page().bounds().unwrap(), (10, 10));
    assert_eq!(filters.state.as_deref(), Some("ready"));
    assert_eq!(query::<SeedQuery>("").page().bounds().unwrap(), (30, 0));
    rejects::<SeedQuery>("page[size]=invalid");
}

#[test]
fn hot_post_queries_accept_numeric_limits_and_defaults() {
    use community::domain::hot_posts::HotPostsQuery;
    let filters: HotPostsQuery = query("limit=20");
    filters.validate().unwrap();
    assert_eq!(filters.limit, 20);
    assert_eq!(query::<HotPostsQuery>("").limit, 10);
    rejects::<HotPostsQuery>("limit=invalid");
}

#[test]
fn operation_queries_accept_defaults_and_time_filters() {
    for defaults in [true, false] {
        let filters: operations::domain::Query = query(&format!(
            "page[number]=2&page[size]=10&search=test&status=normal&strategy=daily&action=none&feedback_type=other&contact=test&user=test&starts_at=2026-11-01T00%3A00%3A00%2B08%3A00&ends_at=2026-11-23T00%3A00%3A00Z&defaults={defaults}"
        ));
        assert_eq!(filters.page().bounds().unwrap(), (10, 10));
        assert_eq!(filters.defaults, defaults);
        assert!(filters.starts_at.is_some());
    }
    assert!(!query::<operations::domain::Query>("").defaults);
    rejects::<operations::domain::Query>("defaults=invalid");
}

#[test]
fn operation_service_catalog_queries_accept_uuid_and_time_filters() {
    use operations::domain::services::CatalogQuery;
    let filters: CatalogQuery = query(
        "page[number]=2&page[size]=10&search=test&category_id=01a201bc-0058-7006-9181-9e875955c7e4&at=2026-11-23T00%3A00%3A00%2B08%3A00",
    );
    assert_eq!(filters.page().bounds().unwrap(), (10, 10));
    assert!(filters.category_id.is_some());
    assert!(filters.at.is_some());
    assert_eq!(query::<CatalogQuery>("").page().bounds().unwrap(), (30, 0));
    rejects::<CatalogQuery>("category_id=invalid");
}

#[test]
fn miniapp_user_queries_accept_boolean_uuid_and_time_filters() {
    use miniapp_users::domain::Query as UserQuery;
    let filters: UserQuery = query(
        "page[number]=2&page[size]=10&search=test&mobile=13800138000&status=normal&created_from=2026-11-01T00%3A00%3A00%2B08%3A00&created_to=2026-11-23T00%3A00%3A00Z&sort=created_at&school_id=01a201bc-0058-7006-9181-9e875955c7e4&unbound=false&account_kind=ordinary",
    );
    assert_eq!(filters.bounds().unwrap(), (10, 10));
    assert_eq!(filters.unbound, Some(false));
    assert!(filters.school_id.is_some());
    assert_eq!(query::<UserQuery>("").bounds().unwrap(), (30, 0));
    rejects::<UserQuery>("unbound=invalid");
}

#[test]
fn school_change_queries_accept_uuid_and_time_filters() {
    use school_changes::domain::Filters;
    let filters: Filters = query(
        "page[number]=2&page[size]=10&status=pending&account_id=01a201bc-0058-7006-9181-9e875955c7e4&created_from=2026-11-01T00%3A00%3A00%2B08%3A00&created_to=2026-11-23T00%3A00%3A00Z",
    );
    filters.validate().unwrap();
    assert_eq!(filters.page().bounds().unwrap(), (10, 10));
    assert!(filters.account_id.is_some());
    assert_eq!(query::<Filters>("").page().bounds().unwrap(), (30, 0));
    rejects::<Filters>("account_id=invalid");
}

#[test]
fn statistics_queries_accept_all_report_filters() {
    use linjian_api::statistics::domain::Query as StatisticsQuery;
    let filters: StatisticsQuery = query(
        "view=schools&period=custom&anchor_date=2026-11-23&start_date=2026-11-01&end_date=2026-11-23&granularity=day&comparison=previous&school_ids=01a201bc-0058-7006-9181-9e875955c7e4%2C01a201bc-0058-7006-9181-9e875955c7e5&school_status=hidden",
    );
    assert_eq!(filters.view().unwrap(), "schools");
    assert!(filters.school_ids.unwrap().contains(','));
    assert_eq!(query::<StatisticsQuery>("").view().unwrap(), "overview");
    rejects::<StatisticsQuery>("unknown=value");
}
