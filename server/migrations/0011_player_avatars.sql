-- Profile pictures players upload from the app (resized on the phone, ~1024 px JPEG).
-- Kept in the database (small images): nothing to configure or back up separately.
CREATE TABLE player_avatars (
    player_id    UUID PRIMARY KEY REFERENCES players (id) ON DELETE CASCADE,
    content_type TEXT NOT NULL CHECK (content_type IN ('image/jpeg', 'image/png', 'image/webp')),
    data         BYTEA NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Set while the player has a picture; part of its URL so caches see a new upload at once.
ALTER TABLE players ADD COLUMN avatar_updated_at TIMESTAMPTZ;
