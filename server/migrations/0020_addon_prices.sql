-- Prices for add-ons (Bookeo "options"), set by admins in Settings and shown on the website.
-- For reference only: what a booking costs is still the total Bookeo calculates. Keyed like
-- addon_images, so one price covers the add-on on every game.
CREATE TABLE addon_prices (
    addon_key  TEXT PRIMARY KEY,
    amount     NUMERIC(12, 2) NOT NULL CHECK (amount >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
