-- Create revisions table for version history
CREATE TABLE IF NOT EXISTS revisions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_revisions_note_id ON revisions(note_id, created_at DESC);
CREATE INDEX idx_revisions_created_at ON revisions(created_at DESC);

-- Trigger function to automatically create revisions on note updates
CREATE OR REPLACE FUNCTION create_note_revision()
RETURNS TRIGGER AS $$
BEGIN
    -- Only create revision if content actually changed and not a new note
    IF (TG_OP = 'UPDATE' AND OLD.content IS DISTINCT FROM NEW.content) THEN
        INSERT INTO revisions (note_id, content, created_by)
        VALUES (NEW.id, OLD.content, NEW.last_edited_by);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_create_note_revision
    BEFORE UPDATE ON notes
    FOR EACH ROW
    EXECUTE FUNCTION create_note_revision();