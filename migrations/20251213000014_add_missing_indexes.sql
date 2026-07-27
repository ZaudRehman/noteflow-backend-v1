-- Missing indexes for common query patterns

-- Composite index for note access checks (owner + collaborator lookups)
CREATE INDEX IF NOT EXISTS idx_notes_access_check
    ON notes(id, user_id, is_deleted);

-- Composite index for note_tags join lookups
CREATE INDEX IF NOT EXISTS idx_note_tags_note_tag
    ON note_tags(note_id, tag_id);

-- Combined full-text search index (avoids coalesce overhead at query time)
CREATE INDEX IF NOT EXISTS idx_notes_full_text_search
    ON notes USING GIN (to_tsvector('english', coalesce(title, '') || ' ' || coalesce(content, '')));
