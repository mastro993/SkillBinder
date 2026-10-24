-- Each skill's record: the Git tree id of the content it should have, and whether content that
-- differs from it can be restored. Forgetting the reconciled commit rehashes every skill, which
-- writes the records of skills that have none.
ALTER TABLE skills ADD COLUMN recorded_tree TEXT NOT NULL DEFAULT '';
ALTER TABLE skills ADD COLUMN drift TEXT;
DELETE FROM device_settings WHERE key = 'reconciled_head';
