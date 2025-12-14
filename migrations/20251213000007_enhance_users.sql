-- migrations/20251213000007_enhance_users.sql
-- Add user preferences, theme, and password reset

ALTER TABLE users ADD COLUMN IF NOT EXISTS preferences JSONB DEFAULT '{}'::jsonb;
ALTER TABLE users ADD COLUMN IF NOT EXISTS theme VARCHAR(20) DEFAULT 'light' CHECK (theme IN ('light', 'dark', 'auto'));
ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_url TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS reset_token VARCHAR(64);
ALTER TABLE users ADD COLUMN IF NOT EXISTS reset_token_expires TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ;

-- Index for JSONB preferences queries
CREATE INDEX IF NOT EXISTS idx_users_preferences ON users USING GIN(preferences);

-- Index for password reset lookups
CREATE INDEX IF NOT EXISTS idx_users_reset_token ON users(reset_token) WHERE reset_token IS NOT NULL;

-- Function to validate preference JSON structure
CREATE OR REPLACE FUNCTION validate_user_preferences()
RETURNS TRIGGER AS $$
BEGIN
  -- Ensure preferences is always a JSON object, not array
  IF jsonb_typeof(NEW.preferences) != 'object' THEN
    RAISE EXCEPTION 'preferences must be a JSON object';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER ensure_valid_preferences
  BEFORE INSERT OR UPDATE ON users
  FOR EACH ROW
  EXECUTE FUNCTION validate_user_preferences();
