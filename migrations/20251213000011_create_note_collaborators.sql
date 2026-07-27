CREATE TABLE note_collaborators (
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permission VARCHAR(20) NOT NULL DEFAULT 'write'
        CHECK (permission IN ('read', 'write', 'admin')),
    invited_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (note_id, user_id)
);

CREATE INDEX idx_note_collaborators_note_id ON note_collaborators(note_id);
CREATE INDEX idx_note_collaborators_user_id ON note_collaborators(user_id);

CREATE OR REPLACE FUNCTION cleanup_collaborator_sessions()
RETURNS TRIGGER AS $$
BEGIN
    DELETE FROM active_sessions
    WHERE note_id = OLD.note_id AND user_id = OLD.user_id;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_cleanup_collaborator_sessions
    AFTER DELETE ON note_collaborators
    FOR EACH ROW
    EXECUTE FUNCTION cleanup_collaborator_sessions();
