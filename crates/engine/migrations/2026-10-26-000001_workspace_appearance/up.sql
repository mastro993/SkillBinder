-- Workspaces gained a tinted glyph or a stored image icon. An image is referenced by a file
-- under the application's icons directory, never by the file the user picked.
ALTER TABLE workspaces ADD COLUMN color TEXT;
ALTER TABLE workspaces ADD COLUMN icon TEXT NOT NULL DEFAULT 'folder';
ALTER TABLE workspaces ADD COLUMN icon_image TEXT;
