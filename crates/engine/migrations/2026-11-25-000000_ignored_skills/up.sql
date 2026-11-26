-- Ignored skills: found skills the user chose never to be offered, by slug, on this device only
-- and never synced (ADR 0033). `key` is the slug as the library compares slugs, so an ignore
-- covers every spelling a scan reports; `slug` keeps the spelling the user ignored.
CREATE TABLE ignored_skills (
    key TEXT PRIMARY KEY NOT NULL,
    slug TEXT NOT NULL,
    ignored_at BIGINT NOT NULL
);
