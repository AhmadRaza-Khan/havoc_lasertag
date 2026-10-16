-- Add-ons chosen for a booking (Bookeo "options", e.g. "UPGRADE YOUR BATTLE"), as sent to
-- Bookeo: [{"id": "...", "name": "...", "value": "..."}]. Kept so they show in lists and are
-- re-sent when a booking is changed (Bookeo replaces the whole booking on update).
ALTER TABLE bookings ADD COLUMN options JSONB NOT NULL DEFAULT '[]';
