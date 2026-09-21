//! The `crates/db/migrations` schema, as Diesel sees it.
//!
//! Column types mirror the migration exactly: every `TEXT` column is `Text` (JSON payload columns
//! included, serialized by the caller), every `INTEGER` column is `BigInt` except `enabled` and
//! `consumed`, which are `Integer` flags.

diesel::table! {
    device_settings (key) {
        key -> Text,
        value -> Text,
    }
}

diesel::table! {
    idempotency_records (operation_id) {
        operation_id -> Text,
        request_hash -> Text,
        result -> Text,
    }
}

diesel::table! {
    operation_plans (id) {
        id -> Text,
        payload -> Text,
        expires_at -> BigInt,
        consumed -> Integer,
    }
}

diesel::table! {
    scan_roots (id) {
        id -> Text,
        canonical_path -> Text,
        display_path -> Text,
        label -> Text,
        enabled -> Integer,
        created_at -> BigInt,
    }
}

diesel::table! {
    skill_metadata (skill_id) {
        skill_id -> Text,
        description -> Nullable<Text>,
        validation -> Text,
        updated_at -> BigInt,
    }
}

diesel::table! {
    source_observations (id) {
        id -> Integer,
        source -> Text,
        skill_id -> Text,
        digest -> Text,
        warnings -> Text,
        reader_agents -> Text,
        observed_at -> BigInt,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    device_settings,
    idempotency_records,
    operation_plans,
    scan_roots,
    skill_metadata,
    source_observations,
);
