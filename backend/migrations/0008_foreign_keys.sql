ALTER TABLE ONLY accounts
    ADD CONSTRAINT accounts_bound_school_id_fkey FOREIGN KEY (bound_school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY audit_events
    ADD CONSTRAINT audit_events_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY audit_events
    ADD CONSTRAINT audit_events_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY operation_metric_events
    ADD CONSTRAINT banner_metric_events_account_id_fkey FOREIGN KEY (account_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY operation_metric_events
    ADD CONSTRAINT banner_metric_events_resource_id_fkey FOREIGN KEY (resource_id) REFERENCES school_resources(id) DEFERRABLE;

ALTER TABLE ONLY operation_metric_events
    ADD CONSTRAINT banner_metric_events_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY blocked_word_imports
    ADD CONSTRAINT blocked_word_imports_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY blocked_word_imports
    ADD CONSTRAINT blocked_word_imports_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY blocked_words
    ADD CONSTRAINT blocked_words_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY cases
    ADD CONSTRAINT cases_account_id_fkey FOREIGN KEY (account_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY cases
    ADD CONSTRAINT cases_deleted_by_fkey FOREIGN KEY (deleted_by) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY cases
    ADD CONSTRAINT cases_handled_by_fkey FOREIGN KEY (handled_by) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY cases
    ADD CONSTRAINT cases_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY cases
    ADD CONSTRAINT cases_school_id_post_id_fkey FOREIGN KEY (school_id, post_id) REFERENCES posts(school_id, id) DEFERRABLE;

ALTER TABLE ONLY comment_reactions
    ADD CONSTRAINT comment_reactions_account_id_fkey FOREIGN KEY (account_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY comment_reactions
    ADD CONSTRAINT comment_reactions_comment_id_fkey FOREIGN KEY (comment_id) REFERENCES comments(id) DEFERRABLE;

ALTER TABLE ONLY comment_reactions
    ADD CONSTRAINT comment_reactions_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY comments
    ADD CONSTRAINT comments_author_id_fkey FOREIGN KEY (author_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY comments
    ADD CONSTRAINT comments_parent_id_fkey FOREIGN KEY (parent_id) REFERENCES comments(id) DEFERRABLE;

ALTER TABLE ONLY comments
    ADD CONSTRAINT comments_school_id_post_id_fkey FOREIGN KEY (school_id, post_id) REFERENCES posts(school_id, id) DEFERRABLE;

ALTER TABLE ONLY customer_config
    ADD CONSTRAINT customer_config_business_asset_id_fkey FOREIGN KEY (business_asset_id) REFERENCES operational_media(id) DEFERRABLE;

ALTER TABLE ONLY customer_config
    ADD CONSTRAINT customer_config_fallback_asset_id_fkey FOREIGN KEY (fallback_asset_id) REFERENCES operational_media(id) DEFERRABLE;

ALTER TABLE ONLY customer_config
    ADD CONSTRAINT customer_config_join_asset_id_fkey FOREIGN KEY (join_asset_id) REFERENCES operational_media(id) DEFERRABLE;

ALTER TABLE ONLY customer_config
    ADD CONSTRAINT customer_config_service_asset_id_fkey FOREIGN KEY (service_asset_id) REFERENCES operational_media(id) DEFERRABLE;

ALTER TABLE ONLY customer_config
    ADD CONSTRAINT customer_config_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY file_asset_leases
    ADD CONSTRAINT file_asset_leases_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES file_assets(id) DEFERRABLE;

ALTER TABLE ONLY file_asset_leases
    ADD CONSTRAINT file_asset_leases_owner_id_fkey FOREIGN KEY (owner_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY file_asset_references
    ADD CONSTRAINT file_asset_references_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES file_assets(id) DEFERRABLE;

ALTER TABLE ONLY file_assets
    ADD CONSTRAINT file_assets_owner_id_fkey FOREIGN KEY (owner_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY file_assets
    ADD CONSTRAINT file_assets_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY follows
    ADD CONSTRAINT follows_follower_id_fkey FOREIGN KEY (follower_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY follows
    ADD CONSTRAINT follows_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY follows
    ADD CONSTRAINT follows_target_id_fkey FOREIGN KEY (target_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY forum_outbox
    ADD CONSTRAINT forum_outbox_binding_version_fkey FOREIGN KEY (binding_version) REFERENCES school_source_binding_versions(id) DEFERRABLE;

ALTER TABLE ONLY forum_outbox
    ADD CONSTRAINT forum_outbox_connection_version_fkey FOREIGN KEY (connection_version) REFERENCES source_connection_versions(id) DEFERRABLE;

ALTER TABLE ONLY forum_outbox
    ADD CONSTRAINT forum_outbox_post_id_fkey FOREIGN KEY (post_id) REFERENCES posts(id) DEFERRABLE;

ALTER TABLE ONLY forum_outbox
    ADD CONSTRAINT forum_outbox_publication_snapshot_id_fkey FOREIGN KEY (publication_snapshot_id) REFERENCES forum_publication_snapshots(id) DEFERRABLE;

ALTER TABLE ONLY forum_outbox
    ADD CONSTRAINT forum_outbox_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY forum_post_recommendations
    ADD CONSTRAINT forum_post_recommendations_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id);

ALTER TABLE ONLY forum_post_recommendations
    ADD CONSTRAINT forum_post_recommendations_event_id_fkey FOREIGN KEY (event_id) REFERENCES forum_outbox(id);

ALTER TABLE ONLY forum_post_recommendations
    ADD CONSTRAINT forum_post_recommendations_post_id_fkey FOREIGN KEY (post_id) REFERENCES posts(id);

ALTER TABLE ONLY forum_post_recommendations
    ADD CONSTRAINT forum_post_recommendations_publication_snapshot_id_fkey FOREIGN KEY (publication_snapshot_id) REFERENCES forum_publication_snapshots(id);

ALTER TABLE ONLY forum_post_recommendations
    ADD CONSTRAINT forum_post_recommendations_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id);

ALTER TABLE ONLY forum_publication_preparations
    ADD CONSTRAINT forum_publication_preparations_event_id_fkey FOREIGN KEY (event_id) REFERENCES forum_outbox(id) DEFERRABLE;

ALTER TABLE ONLY forum_publication_preparations
    ADD CONSTRAINT forum_publication_preparations_link_asset_id_fkey FOREIGN KEY (link_asset_id) REFERENCES miniapp_link_assets(id) DEFERRABLE;

ALTER TABLE ONLY forum_publication_preparations
    ADD CONSTRAINT forum_publication_preparations_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES forum_publication_snapshots(id) DEFERRABLE;

ALTER TABLE ONLY forum_publication_snapshots
    ADD CONSTRAINT forum_publication_snapshots_author_id_fkey FOREIGN KEY (author_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY forum_publication_snapshots
    ADD CONSTRAINT forum_publication_snapshots_moderation_event_id_fkey FOREIGN KEY (moderation_event_id) REFERENCES moderation_events(id) DEFERRABLE;

ALTER TABLE ONLY forum_publication_snapshots
    ADD CONSTRAINT forum_publication_snapshots_moderator_id_fkey FOREIGN KEY (moderator_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY forum_publication_snapshots
    ADD CONSTRAINT forum_publication_snapshots_school_id_post_id_fkey FOREIGN KEY (school_id, post_id) REFERENCES posts(school_id, id) DEFERRABLE;

ALTER TABLE ONLY forum_source_terminations
    ADD CONSTRAINT forum_source_terminations_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id);

ALTER TABLE ONLY forum_source_terminations
    ADD CONSTRAINT forum_source_terminations_event_id_fkey FOREIGN KEY (event_id) REFERENCES forum_outbox(id);

ALTER TABLE ONLY forum_source_versions
    ADD CONSTRAINT forum_source_versions_post_id_fkey FOREIGN KEY (post_id) REFERENCES posts(id) DEFERRABLE;

ALTER TABLE ONLY forum_source_versions
    ADD CONSTRAINT forum_source_versions_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY home_menu_imports
    ADD CONSTRAINT home_menu_imports_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY home_menu_imports
    ADD CONSTRAINT home_menu_imports_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY home_menu_settings
    ADD CONSTRAINT home_menu_settings_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY home_menu_tags
    ADD CONSTRAINT home_menu_tags_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY marketing_events
    ADD CONSTRAINT marketing_events_account_id_fkey FOREIGN KEY (account_id) REFERENCES accounts(id);

ALTER TABLE ONLY marketing_events
    ADD CONSTRAINT marketing_events_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id);

ALTER TABLE ONLY marketing_events
    ADD CONSTRAINT marketing_events_slot_id_fkey FOREIGN KEY (slot_id) REFERENCES marketing_slots(id);

ALTER TABLE ONLY media_access_tokens
    ADD CONSTRAINT media_access_tokens_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES media_assets(id) DEFERRABLE;

ALTER TABLE ONLY media_access_tokens
    ADD CONSTRAINT media_access_tokens_post_id_fkey FOREIGN KEY (post_id) REFERENCES posts(id) DEFERRABLE;

ALTER TABLE ONLY media_access_tokens
    ADD CONSTRAINT media_access_tokens_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY media_assets
    ADD CONSTRAINT media_assets_id_fkey FOREIGN KEY (id) REFERENCES file_assets(id) DEFERRABLE;

ALTER TABLE ONLY media_assets
    ADD CONSTRAINT media_assets_owner_id_fkey FOREIGN KEY (owner_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY media_assets
    ADD CONSTRAINT media_assets_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY miniapp_avatar_assets
    ADD CONSTRAINT miniapp_avatar_assets_id_fkey FOREIGN KEY (id) REFERENCES file_assets(id) DEFERRABLE;

ALTER TABLE ONLY miniapp_avatar_assets
    ADD CONSTRAINT miniapp_avatar_assets_owner_id_fkey FOREIGN KEY (owner_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY miniapp_identities
    ADD CONSTRAINT miniapp_identities_account_id_fkey FOREIGN KEY (account_id) REFERENCES miniapp_users(account_id) DEFERRABLE;

ALTER TABLE ONLY miniapp_registration_settings
    ADD CONSTRAINT miniapp_registration_settings_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES accounts(id) ON DELETE SET NULL;

ALTER TABLE ONLY miniapp_school_change_documents
    ADD CONSTRAINT miniapp_school_change_documents_owner_id_fkey FOREIGN KEY (owner_id) REFERENCES miniapp_users(account_id);

ALTER TABLE ONLY miniapp_school_change_documents
    ADD CONSTRAINT miniapp_school_change_documents_owner_id_request_id_fkey FOREIGN KEY (owner_id, request_id) REFERENCES miniapp_school_change_requests(account_id, id);

ALTER TABLE ONLY miniapp_school_change_events
    ADD CONSTRAINT miniapp_school_change_events_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id);

ALTER TABLE ONLY miniapp_school_change_events
    ADD CONSTRAINT miniapp_school_change_events_document_id_fkey FOREIGN KEY (document_id) REFERENCES miniapp_school_change_documents(id);

ALTER TABLE ONLY miniapp_school_change_events
    ADD CONSTRAINT miniapp_school_change_events_request_id_fkey FOREIGN KEY (request_id) REFERENCES miniapp_school_change_requests(id);

ALTER TABLE ONLY miniapp_school_change_requests
    ADD CONSTRAINT miniapp_school_change_requests_account_id_fkey FOREIGN KEY (account_id) REFERENCES miniapp_users(account_id);

ALTER TABLE ONLY miniapp_school_change_requests
    ADD CONSTRAINT miniapp_school_change_requests_from_school_id_fkey FOREIGN KEY (from_school_id) REFERENCES schools(id);

ALTER TABLE ONLY miniapp_school_change_requests
    ADD CONSTRAINT miniapp_school_change_requests_reviewed_by_fkey FOREIGN KEY (reviewed_by) REFERENCES accounts(id);

ALTER TABLE ONLY miniapp_school_change_requests
    ADD CONSTRAINT miniapp_school_change_requests_to_school_id_fkey FOREIGN KEY (to_school_id) REFERENCES schools(id);

ALTER TABLE ONLY miniapp_users
    ADD CONSTRAINT miniapp_users_account_id_avatar_asset_id_fkey FOREIGN KEY (account_id, avatar_asset_id) REFERENCES miniapp_avatar_assets(owner_id, id) DEFERRABLE;

ALTER TABLE ONLY miniapp_users
    ADD CONSTRAINT miniapp_users_account_id_fkey FOREIGN KEY (account_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY miniapp_users
    ADD CONSTRAINT miniapp_users_status_changed_by_fkey FOREIGN KEY (status_changed_by) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY moderation_events
    ADD CONSTRAINT moderation_events_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY moderation_events
    ADD CONSTRAINT moderation_events_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY notifications
    ADD CONSTRAINT notifications_account_id_fkey FOREIGN KEY (account_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY notifications
    ADD CONSTRAINT notifications_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY operation_action_events
    ADD CONSTRAINT operation_action_events_click_id_fkey FOREIGN KEY (click_id) REFERENCES operation_metric_events(event_id) ON DELETE CASCADE;

ALTER TABLE ONLY operation_cooperations
    ADD CONSTRAINT operation_cooperations_partner_id_fkey FOREIGN KEY (partner_id) REFERENCES operation_partners(id);

ALTER TABLE ONLY operation_daily_metrics
    ADD CONSTRAINT operation_daily_metrics_resource_id_fkey FOREIGN KEY (resource_id) REFERENCES school_resources(id);

ALTER TABLE ONLY operation_daily_metrics
    ADD CONSTRAINT operation_daily_metrics_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id);

ALTER TABLE ONLY operation_metric_events
    ADD CONSTRAINT operation_metric_events_target_version_id_fkey FOREIGN KEY (target_version_id) REFERENCES operation_target_versions(id);

ALTER TABLE ONLY operation_report_snapshots
    ADD CONSTRAINT operation_report_snapshots_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id);

ALTER TABLE ONLY operation_report_snapshots
    ADD CONSTRAINT operation_report_snapshots_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id);

ALTER TABLE ONLY operation_service_categories
    ADD CONSTRAINT operation_service_categories_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id);

ALTER TABLE ONLY operation_target_reviews
    ADD CONSTRAINT operation_target_reviews_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id);

ALTER TABLE ONLY operation_target_reviews
    ADD CONSTRAINT operation_target_reviews_version_id_fkey FOREIGN KEY (version_id) REFERENCES operation_target_versions(id);

ALTER TABLE ONLY operation_target_versions
    ADD CONSTRAINT operation_target_versions_creator_id_fkey FOREIGN KEY (creator_id) REFERENCES accounts(id);

ALTER TABLE ONLY operation_target_versions
    ADD CONSTRAINT operation_target_versions_submitter_id_fkey FOREIGN KEY (submitter_id) REFERENCES accounts(id);

ALTER TABLE ONLY operation_target_versions
    ADD CONSTRAINT operation_target_versions_target_id_fkey FOREIGN KEY (target_id) REFERENCES operation_service_targets(id);

ALTER TABLE ONLY operational_media
    ADD CONSTRAINT operational_media_id_fkey FOREIGN KEY (id) REFERENCES file_assets(id) DEFERRABLE;

ALTER TABLE ONLY operational_media
    ADD CONSTRAINT operational_media_owner_id_fkey FOREIGN KEY (owner_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY operational_media
    ADD CONSTRAINT operational_media_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY post_import_jobs
    ADD CONSTRAINT post_import_jobs_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY post_import_jobs
    ADD CONSTRAINT post_import_jobs_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY post_order_versions
    ADD CONSTRAINT post_order_versions_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY post_reactions
    ADD CONSTRAINT post_reactions_account_id_fkey FOREIGN KEY (account_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY post_reactions
    ADD CONSTRAINT post_reactions_school_id_post_id_fkey FOREIGN KEY (school_id, post_id) REFERENCES posts(school_id, id) DEFERRABLE;

ALTER TABLE ONLY post_seed_batches
    ADD CONSTRAINT post_seed_batches_created_by_fkey FOREIGN KEY (created_by) REFERENCES accounts(id);

ALTER TABLE ONLY post_seed_batches
    ADD CONSTRAINT post_seed_batches_revoked_by_fkey FOREIGN KEY (revoked_by) REFERENCES accounts(id);

ALTER TABLE ONLY post_seed_batches
    ADD CONSTRAINT post_seed_batches_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id);

ALTER TABLE ONLY post_seed_items
    ADD CONSTRAINT post_seed_items_author_id_fkey FOREIGN KEY (author_id) REFERENCES accounts(id);

ALTER TABLE ONLY post_seed_items
    ADD CONSTRAINT post_seed_items_school_id_batch_id_fkey FOREIGN KEY (school_id, batch_id) REFERENCES post_seed_batches(school_id, id);

ALTER TABLE ONLY post_seed_items
    ADD CONSTRAINT post_seed_items_school_id_post_id_fkey FOREIGN KEY (school_id, post_id) REFERENCES posts(school_id, id);

ALTER TABLE ONLY posts
    ADD CONSTRAINT posts_author_id_fkey FOREIGN KEY (author_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY posts
    ADD CONSTRAINT posts_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY posts
    ADD CONSTRAINT posts_school_id_id_approved_snapshot_id_fkey FOREIGN KEY (school_id, id, approved_snapshot_id) REFERENCES forum_publication_snapshots(school_id, post_id, id) DEFERRABLE;

ALTER TABLE ONLY posts
    ADD CONSTRAINT posts_school_id_seed_batch_id_fkey FOREIGN KEY (school_id, seed_batch_id) REFERENCES post_seed_batches(school_id, id);

ALTER TABLE ONLY product_previews
    ADD CONSTRAINT product_previews_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY product_previews
    ADD CONSTRAINT product_previews_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY resource_publications
    ADD CONSTRAINT resource_publications_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY resource_publications
    ADD CONSTRAINT resource_publications_resource_id_fkey FOREIGN KEY (resource_id) REFERENCES school_resources(id) DEFERRABLE;

ALTER TABLE ONLY resource_publications
    ADD CONSTRAINT resource_publications_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY school_authorization_guards
    ADD CONSTRAINT school_authorization_guards_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id);

ALTER TABLE ONLY school_marketing_settings
    ADD CONSTRAINT school_marketing_settings_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id);

ALTER TABLE ONLY school_marketing_settings
    ADD CONSTRAINT school_marketing_settings_slot_id_fkey FOREIGN KEY (slot_id) REFERENCES marketing_slots(id);

ALTER TABLE ONLY school_member_authorizations
    ADD CONSTRAINT school_member_authorizations_account_id_fkey FOREIGN KEY (account_id) REFERENCES accounts(id);

ALTER TABLE ONLY school_member_authorizations
    ADD CONSTRAINT school_member_authorizations_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id);

ALTER TABLE ONLY school_member_authorizations
    ADD CONSTRAINT school_member_authorizations_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES accounts(id);

ALTER TABLE ONLY school_member_permission_overrides
    ADD CONSTRAINT school_member_permission_overrides_school_id_account_id_fkey FOREIGN KEY (school_id, account_id) REFERENCES school_member_authorizations(school_id, account_id);

ALTER TABLE ONLY school_member_permission_overrides
    ADD CONSTRAINT school_member_permission_overrides_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES accounts(id);

ALTER TABLE ONLY school_memberships
    ADD CONSTRAINT school_memberships_account_id_fkey FOREIGN KEY (account_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY school_memberships
    ADD CONSTRAINT school_memberships_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY school_resources
    ADD CONSTRAINT school_resources_category_id_fkey FOREIGN KEY (category_id) REFERENCES operation_service_categories(id);

ALTER TABLE ONLY school_resources
    ADD CONSTRAINT school_resources_cooperation_id_fkey FOREIGN KEY (cooperation_id) REFERENCES operation_cooperations(id);

ALTER TABLE ONLY school_resources
    ADD CONSTRAINT school_resources_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY school_resources
    ADD CONSTRAINT school_resources_target_version_id_fkey FOREIGN KEY (target_version_id) REFERENCES operation_target_versions(id);

ALTER TABLE ONLY school_shortcut_overrides
    ADD CONSTRAINT school_shortcut_overrides_resource_id_fkey FOREIGN KEY (resource_id) REFERENCES school_resources(id);

ALTER TABLE ONLY school_shortcut_overrides
    ADD CONSTRAINT school_shortcut_overrides_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id);

ALTER TABLE ONLY school_source_binding_versions
    ADD CONSTRAINT school_source_binding_versions_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY school_source_binding_versions
    ADD CONSTRAINT school_source_binding_versions_connection_version_fkey FOREIGN KEY (connection_version) REFERENCES source_connection_versions(id) DEFERRABLE;

ALTER TABLE ONLY school_source_binding_versions
    ADD CONSTRAINT school_source_binding_versions_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY school_source_bindings
    ADD CONSTRAINT school_source_bindings_connection_id_fkey FOREIGN KEY (connection_id) REFERENCES source_connections(id) DEFERRABLE;

ALTER TABLE ONLY school_source_bindings
    ADD CONSTRAINT school_source_bindings_connection_version_connection_id_so_fkey FOREIGN KEY (connection_version, connection_id, source_id, origin) REFERENCES source_connection_versions(id, connection_id, source_id, origin) DEFERRABLE;

ALTER TABLE ONLY school_source_bindings
    ADD CONSTRAINT school_source_bindings_connection_version_fkey FOREIGN KEY (connection_version) REFERENCES source_connection_versions(id) DEFERRABLE;

ALTER TABLE ONLY school_source_bindings
    ADD CONSTRAINT school_source_bindings_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY schools
    ADD CONSTRAINT schools_id_header_bg_asset_id_fkey FOREIGN KEY (id, header_bg_asset_id) REFERENCES operational_media(school_id, id) DEFERRABLE;

ALTER TABLE ONLY schools
    ADD CONSTRAINT schools_id_logo_asset_id_fkey FOREIGN KEY (id, logo_asset_id) REFERENCES operational_media(school_id, id) DEFERRABLE;

ALTER TABLE ONLY sessions
    ADD CONSTRAINT sessions_account_id_fkey FOREIGN KEY (account_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY site_settings
    ADD CONSTRAINT site_settings_logo_asset_id_fkey FOREIGN KEY (logo_asset_id) REFERENCES operational_media(id);

ALTER TABLE ONLY site_settings
    ADD CONSTRAINT site_settings_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES accounts(id);

ALTER TABLE ONLY source_active_configuration
    ADD CONSTRAINT source_active_configuration_connection_id_fkey FOREIGN KEY (connection_id) REFERENCES source_connections(id) DEFERRABLE;

ALTER TABLE ONLY source_connection_versions
    ADD CONSTRAINT source_connection_versions_connection_id_fkey FOREIGN KEY (connection_id) REFERENCES source_connections(id) DEFERRABLE;

ALTER TABLE ONLY source_connection_versions
    ADD CONSTRAINT source_connection_versions_connection_id_source_id_origin_fkey FOREIGN KEY (connection_id, source_id, origin) REFERENCES source_connections(id, source_id, origin) DEFERRABLE;

ALTER TABLE ONLY source_import_batches
    ADD CONSTRAINT source_import_batches_id_fkey FOREIGN KEY (id) REFERENCES migration_runs(id) DEFERRABLE;

ALTER TABLE ONLY source_imported_posts
    ADD CONSTRAINT source_imported_posts_batch_id_fkey FOREIGN KEY (batch_id) REFERENCES source_import_batches(id) DEFERRABLE;

ALTER TABLE ONLY source_imported_posts
    ADD CONSTRAINT source_imported_posts_post_id_fkey FOREIGN KEY (post_id) REFERENCES posts(id) DEFERRABLE;

ALTER TABLE ONLY statistics_activity_events
    ADD CONSTRAINT statistics_activity_events_account_id_fkey FOREIGN KEY (account_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY statistics_activity_events
    ADD CONSTRAINT statistics_activity_events_post_id_fkey FOREIGN KEY (post_id) REFERENCES posts(id) DEFERRABLE;

ALTER TABLE ONLY statistics_activity_events
    ADD CONSTRAINT statistics_activity_events_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY statistics_registrations
    ADD CONSTRAINT statistics_registrations_account_id_fkey FOREIGN KEY (account_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY statistics_registrations
    ADD CONSTRAINT statistics_registrations_registration_school_id_fkey FOREIGN KEY (registration_school_id) REFERENCES schools(id) DEFERRABLE;

ALTER TABLE ONLY statistics_snapshots
    ADD CONSTRAINT statistics_snapshots_owner_id_fkey FOREIGN KEY (owner_id) REFERENCES accounts(id) DEFERRABLE;

ALTER TABLE ONLY statistics_snapshots
    ADD CONSTRAINT statistics_snapshots_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools(id) DEFERRABLE;
