-- The last import's result, which Discovery's banner showed until dismissed. Imports end on the
-- progress strip now; the result stays in idempotency_records for replay.
DELETE FROM device_settings WHERE key = 'import_outcome';
