-- Text promotions shown in the mobile app. An offer is live while it's active and today
-- (in the venue's timezone) falls within its optional start/end dates, inclusive.
CREATE TABLE offers (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title       TEXT NOT NULL,
    description TEXT NOT NULL,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    starts_on   DATE,
    ends_on     DATE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT offers_dates_ordered CHECK (starts_on IS NULL OR ends_on IS NULL OR starts_on <= ends_on)
);

CREATE INDEX offers_created_at_idx ON offers (created_at DESC);
