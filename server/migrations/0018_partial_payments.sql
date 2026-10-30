-- Partial payments: a booking can be paid in several payments (at least QAR 120 each, or the
-- rest if less), by its player or by the other players on it.

-- partially_paid: something was paid, the rest can be paid online until the game starts
-- (after that, at the venue). Such a booking is never canceled for non-payment.
ALTER TABLE bookings DROP CONSTRAINT bookings_payment_status_check;
ALTER TABLE bookings ADD CONSTRAINT bookings_payment_status_check
    CHECK (payment_status IN ('not_required', 'unpaid', 'partially_paid', 'paid'));

-- Sum of the booking's paid payments, kept up to date as payments arrive.
ALTER TABLE bookings ADD COLUMN amount_paid NUMERIC(12, 2) NOT NULL DEFAULT 0;
UPDATE bookings b SET amount_paid = p.total
FROM (SELECT booking_id, SUM(amount::numeric) AS total FROM payments WHERE status = 'paid' GROUP BY booking_id) p
WHERE p.booking_id = b.id;

-- A booking may now have many paid payments.
DROP INDEX payments_one_paid_per_booking;

-- Who paid (the booking's player or one of its other players). Kept if they delete their account.
ALTER TABLE payments ADD COLUMN payer_id UUID REFERENCES players (id) ON DELETE SET NULL;
UPDATE payments p SET payer_id = b.player_id FROM bookings b WHERE b.id = p.booking_id;
CREATE INDEX payments_payer_idx ON payments (booking_id, payer_id) WHERE status = 'open';
