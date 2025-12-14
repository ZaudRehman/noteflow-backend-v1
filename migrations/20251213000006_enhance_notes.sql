-- migrations/20251213000006_enhance_notes.sql
-- Add favorite and archive functionality

ALTER TABLE notes ADD COLUMN IF NOT EXISTS is_favorited BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE notes ADD COLUMN IF NOT EXISTS is_archived BOOLEAN NOT NULL DEFAULT FALSE;

-- Create partial indexes for performance
CREATE INDEX IF NOT EXISTS idx_notes_user_favorited 
ON notes(user_id, updated_at DESC) 
WHERE is_favorited = true AND is_deleted = false;

CREATE INDEX IF NOT EXISTS idx_notes_user_archived 
ON notes(user_id, updated_at DESC) 
WHERE is_archived = true AND is_deleted = false;

-- Composite index for filtered queries
CREATE INDEX IF NOT EXISTS idx_notes_user_filters 
ON notes(user_id, is_deleted, is_archived, is_favorited, updated_at DESC);

-- Add comment for documentation
COMMENT ON COLUMN notes.is_favorited IS 'User-marked favorite notes';
COMMENT ON COLUMN notes.is_archived IS 'Archived notes (not shown in main list)';
