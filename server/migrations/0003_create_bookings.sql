-- Players are mirrored as Bookeo customers on their first booking.
ALTER TABLE players ADD COLUMN bookeo_customer_id TEXT;

-- Bookings made by players through the app. Bookeo remains the source of truth;
-- this table links each Bookeo booking to the player who made it.
CREATE TABLE bookings (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_id             UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    bookeo_booking_number TEXT NOT NULL UNIQUE,
    product_id            TEXT NOT NULL,
    product_name          TEXT NOT NULL,
    event_id              TEXT NOT NULL,
    start_time            TIMESTAMPTZ NOT NULL,
    end_time              TIMESTAMPTZ NOT NULL,
    num_people            INTEGER NOT NULL CHECK (num_people > 0),
    -- pending: awaiting acceptance by staff (products with accept/deny enabled)
    status                TEXT NOT NULL CHECK (status IN ('confirmed', 'pending', 'canceled')),
    price_amount          TEXT,
    price_currency        TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX bookings_player_start_idx ON bookings (player_id, start_time DESC);
