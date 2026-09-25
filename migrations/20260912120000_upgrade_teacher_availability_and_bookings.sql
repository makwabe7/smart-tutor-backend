-- Canonical availability and appointment fields. Existing rows remain readable
-- through the legacy fields while new bookings use starts_at/ends_at.
CREATE EXTENSION IF NOT EXISTS btree_gist;

ALTER TABLE teacher_availability
    DROP CONSTRAINT IF EXISTS teacher_availability_unique,
    ADD COLUMN IF NOT EXISTS slot_duration_minutes INTEGER NOT NULL DEFAULT 60
        CHECK (slot_duration_minutes IN (30, 45, 60, 90)),
    ADD COLUMN IF NOT EXISTS session_type VARCHAR(12) NOT NULL DEFAULT 'both'
        CHECK (session_type IN ('online', 'physical', 'both')),
    ADD COLUMN IF NOT EXISTS subject VARCHAR(120),
    ADD COLUMN IF NOT EXISTS grade_level VARCHAR(120),
    ADD COLUMN IF NOT EXISTS available_from DATE NOT NULL DEFAULT CURRENT_DATE,
    ADD COLUMN IF NOT EXISTS available_until DATE,
    ADD COLUMN IF NOT EXISTS repeats_weekly BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD CONSTRAINT teacher_availability_date_range_check
        CHECK (available_until IS NULL OR available_until >= available_from);

ALTER TABLE tutor_bookings
    ADD COLUMN IF NOT EXISTS availability_id UUID REFERENCES teacher_availability(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS starts_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS ends_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS session_type VARCHAR(12),
    ADD COLUMN IF NOT EXISTS subject VARCHAR(120),
    ADD COLUMN IF NOT EXISTS grade_level VARCHAR(120),
    ADD COLUMN IF NOT EXISTS notes TEXT,
    ADD COLUMN IF NOT EXISTS meeting_url TEXT,
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

ALTER TABLE tutor_bookings
    DROP CONSTRAINT IF EXISTS tutor_bookings_status_check,
    ADD CONSTRAINT tutor_bookings_status_check
        CHECK (status IN ('pending', 'confirmed', 'cancelled', 'completed', 'rejected')),
    ADD CONSTRAINT tutor_bookings_timestamp_range_check
        CHECK (starts_at IS NULL OR (ends_at IS NOT NULL AND ends_at > starts_at)),
    ADD CONSTRAINT tutor_bookings_session_type_check
        CHECK (session_type IS NULL OR session_type IN ('online', 'physical'));

CREATE INDEX IF NOT EXISTS tutor_bookings_tutor_starts_at_idx
    ON tutor_bookings (tutor_id, starts_at)
    WHERE status IN ('pending', 'confirmed');

-- This is the final authority against double booking. It is safe even if two
-- booking requests pass application validation at the same time.
ALTER TABLE tutor_bookings
    ADD CONSTRAINT tutor_bookings_no_teacher_overlap
    EXCLUDE USING gist (
        tutor_id WITH =,
        tstzrange(starts_at, ends_at, '[)') WITH &&
    ) WHERE (status IN ('pending', 'confirmed'));
