-- Pictures for add-ons (Bookeo "options"), set by admins in Settings. Bookeo has none.
-- Bookeo lists a global add-on on every game with a game-specific id ("<product>_<option>"); the
-- part after the last "_" is the same on every game, so one picture covers them all.
CREATE TABLE addon_images (
    addon_key    TEXT PRIMARY KEY,
    content_type TEXT NOT NULL CHECK (content_type IN ('image/jpeg', 'image/png', 'image/webp')),
    data         BYTEA NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
