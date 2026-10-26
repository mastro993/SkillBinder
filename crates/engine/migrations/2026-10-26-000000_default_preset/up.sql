-- Every library starts with its default preset, at the front of any presets it already has.
UPDATE presets SET position = position + 1;

-- A preset that already carries the default name becomes the default one, keeping its color and
-- skills, so the library never holds two presets with that name.
INSERT INTO presets(id, name, position, color, icon)
SELECT 'default', 'Default', 0, color, 'layers'
FROM presets
WHERE lower(trim(name)) = 'default'
ORDER BY position
LIMIT 1;
INSERT INTO preset_skills(preset_id, slug)
SELECT 'default', slug
FROM preset_skills
WHERE preset_id = (
    SELECT id FROM presets WHERE lower(trim(name)) = 'default' AND id <> 'default' LIMIT 1
);
DELETE FROM presets WHERE lower(trim(name)) = 'default' AND id <> 'default';

-- Otherwise the library gets its default preset here.
INSERT INTO presets(id, name, position, color, icon)
SELECT 'default', 'Default', 0, NULL, 'layers'
WHERE NOT EXISTS (SELECT 1 FROM presets WHERE id = 'default');

-- The folder glyph left the curated preset icons; presets that showed it show the default glyph.
UPDATE presets SET icon = 'layers' WHERE icon = 'folder';
