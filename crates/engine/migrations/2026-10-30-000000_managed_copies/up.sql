-- A preset holds each skill for all your agents, or, when `agents` holds a JSON array of registry
-- identities, for exactly those agents.
ALTER TABLE preset_skills ADD COLUMN agents TEXT;

-- The copies SkillBinder wrote or adopted in your agents' global skill directories, by resolved
-- directory and slug, with the manifest digest each had when it was written or adopted, and the
-- directories where an active skill is not applied, or a managed copy was not removed, and why.
CREATE TABLE managed_copies(
    directory TEXT NOT NULL,
    slug TEXT NOT NULL,
    digest TEXT,
    issue TEXT,
    PRIMARY KEY(directory, slug)
);

-- The engine applies the active preset, so the choice moves from the window's preferences into
-- its own setting. A preset that no longer exists leaves the Default preset active.
INSERT INTO device_settings(key, value)
SELECT 'active_preset', json_quote(json_extract(value, '$.active_preset'))
FROM device_settings
WHERE key = 'preferences'
    AND json_extract(value, '$.active_preset') IN (SELECT id FROM presets);
