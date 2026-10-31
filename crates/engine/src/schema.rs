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
    workspaces (id) {
        id -> Text,
        canonical_path -> Text,
        name -> Text,
        position -> BigInt,
        created_at -> BigInt,
        color -> Nullable<Text>,
        icon -> Text,
        icon_image -> Nullable<Text>,
    }
}

diesel::table! {
    library_folders (id) {
        id -> Text,
        name -> Text,
        position -> BigInt,
    }
}

diesel::table! {
    skills (slug) {
        slug -> Text,
        digest -> Text,
        file_count -> BigInt,
        total_bytes -> BigInt,
        details -> Text,
        imported_at -> Nullable<Text>,
        folder_id -> Nullable<Text>,
        source -> Nullable<Text>,
        recorded_tree -> Text,
        drift -> Nullable<Text>,
    }
}

diesel::table! {
    presets (id) {
        id -> Text,
        name -> Text,
        position -> BigInt,
        color -> Nullable<Text>,
        icon -> Text,
    }
}

diesel::table! {
    preset_skills (preset_id, slug) {
        preset_id -> Text,
        slug -> Text,
        agents -> Nullable<Text>,
    }
}

diesel::table! {
    managed_copies (directory, slug) {
        directory -> Text,
        slug -> Text,
        digest -> Nullable<Text>,
        issue -> Nullable<Text>,
        workspace_id -> Nullable<Text>,
    }
}

diesel::table! {
    workspace_dirs (workspace_id, dir) {
        workspace_id -> Text,
        dir -> Text,
    }
}

diesel::table! {
    workspace_skills (workspace_id, slug) {
        workspace_id -> Text,
        slug -> Text,
        dirs -> Nullable<Text>,
    }
}

diesel::table! {
    journals (id) {
        id -> Text,
        body -> Text,
    }
}
