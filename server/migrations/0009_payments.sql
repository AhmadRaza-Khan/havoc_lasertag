-- Online payment of app bookings through QNB's hosted payment page (Pay by Link).

-- Admin-editable payment settings. Exactly one row.
CREATE TABLE payment_settings (
    id                 BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
    -- pay_now: pay right after booking (unpaid bookings are canceled after a short window).
    -- pay_later: pay within pay_later_minutes of booking, or the booking is canceled.
    mode               TEXT NOT NULL DEFAULT 'pay_now' CHECK (mode IN ('pay_now', 'pay_later')),
    pay_later_minutes  INTEGER NOT NULL DEFAULT 1440 CHECK (pay_later_minutes BETWEEN 15 AND 43200),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO payment_settings DEFAULT VALUES;

-- not_required: nothing to pay online (admin-made bookings, free games, payments disabled).
ALTER TABLE bookings
    ADD COLUMN payment_status TEXT NOT NULL DEFAULT 'not_required'
        CHECK (payment_status IN ('not_required', 'unpaid', 'paid')),
    ADD COLUMN payment_mode   TEXT CHECK (payment_mode IN ('pay_now', 'pay_later')),
    -- Unpaid bookings are canceled once this passes.
    ADD COLUMN payment_due_at TIMESTAMPTZ;

CREATE INDEX bookings_payment_due_idx ON bookings (payment_due_at)
    WHERE payment_status = 'unpaid' AND status <> 'canceled';

-- QNB transaction references must be unique and at most 20 characters.
CREATE SEQUENCE payment_txn_seq;

-- One row per QNB payment link. A link stays `open` (payable) until it's paid or we close it.
CREATE TABLE payments (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    booking_id        UUID NOT NULL REFERENCES bookings (id) ON DELETE CASCADE,
    merchant_txn_id   TEXT NOT NULL UNIQUE DEFAULT 'HVC' || lpad(nextval('payment_txn_seq')::text, 12, '0'),
    ipg_txn_id        TEXT UNIQUE,
    pay_url           TEXT,
    amount            TEXT NOT NULL,
    currency          TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'paid', 'closed')),
    -- QNB's answer to the latest attempt on this link (e.g. card declined), for the app.
    last_failure      TEXT,
    paid_at           TIMESTAMPTZ,
    -- Recording the payment in Bookeo; retried until bookeo_payment_id is set.
    bookeo_payment_id TEXT,
    bookeo_sync_at    TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX payments_booking_idx ON payments (booking_id, created_at DESC);
CREATE INDEX payments_open_idx ON payments (created_at) WHERE status = 'open';
-- A booking is paid at most once.
CREATE UNIQUE INDEX payments_one_paid_per_booking ON payments (booking_id) WHERE status = 'paid';
