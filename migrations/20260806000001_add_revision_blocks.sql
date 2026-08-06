-- Revision snapshots for the block-based editor.
-- Store the full block set alongside the plain-text content so a restore
-- can rebuild note_blocks exactly as it was saved.

ALTER TABLE revisions
    ADD COLUMN IF NOT EXISTS blocks JSONB NOT NULL DEFAULT '[]'::jsonb;

-- Replace the content-only trigger with application-managed snapshots.
-- The trigger could only capture plain-text `content` (blocks live in a
-- separate table that is already rewritten by the time the trigger fires),
-- so restores could never rebuild the block editor state.
DROP TRIGGER IF EXISTS trigger_create_note_revision ON notes;
DROP FUNCTION IF EXISTS create_note_revision();
