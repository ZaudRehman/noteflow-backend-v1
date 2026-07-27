-- Collaborative editing operations (CRDT relay buffer)
-- Stores insert/delete operations for real-time collaborative editing.
-- Cleansed periodically by a background task that applies them to notes.content.

CREATE TABLE IF NOT EXISTS collab_operations (
    id BIGSERIAL PRIMARY KEY,
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    client_id TEXT NOT NULL,
    op_type TEXT NOT NULL CHECK (op_type IN ('insert', 'delete')),
    position INT NOT NULL,
    text_content TEXT,
    length INT,
    applied BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_collab_ops_note_pending
    ON collab_operations(note_id, id)
    WHERE NOT applied;

CREATE INDEX IF NOT EXISTS idx_collab_ops_note_all
    ON collab_operations(note_id, id);

-- Trigger-based cleanup: auto-delete applied ops older than 1 hour
CREATE OR REPLACE FUNCTION cleanup_old_collab_ops() RETURNS trigger AS $$
BEGIN
    DELETE FROM collab_operations
    WHERE applied = true AND created_at < NOW() - INTERVAL '1 hour';
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_cleanup_collab_ops ON collab_operations;
CREATE TRIGGER trigger_cleanup_collab_ops
    AFTER INSERT ON collab_operations
    EXECUTE FUNCTION cleanup_old_collab_ops();
