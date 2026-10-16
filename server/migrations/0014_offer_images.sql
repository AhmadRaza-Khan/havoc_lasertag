-- Offers can be text, an image, or both. Title and description may now be empty ('');
-- an offer with neither text nor an image isn't shown in the app.
CREATE TABLE offer_images (
    offer_id     UUID PRIMARY KEY REFERENCES offers (id) ON DELETE CASCADE,
    content_type TEXT NOT NULL CHECK (content_type IN ('image/jpeg', 'image/png', 'image/webp')),
    data         BYTEA NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Set while the offer has an image; part of its URL so caches see a new upload at once.
ALTER TABLE offers ADD COLUMN image_updated_at TIMESTAMPTZ;
