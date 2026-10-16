//! Diesel table definitions matching `migrations/`.

diesel::table! {
    device_settings (key) {
        key -> Text,
        value -> Text,
    }
}

diesel::table! {
    operation_plans (id) {
        id -> Text,
        payload -> Text,
        expires_at -> BigInt,
        consumed -> BigInt,
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
    skill_metadata (skill_id) {
        skill_id -> Text,
        description -> Text,
        validation -> Text,
        updated_at -> BigInt,
    }
}

diesel::table! {
    workspaces (id) {
        id -> Text,
        canonical_path -> Text,
        name -> Text,
        position -> BigInt,
        created_at -> BigInt,
    }
}
