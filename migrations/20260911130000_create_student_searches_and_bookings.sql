CREATE TABLE student_tutor_searches (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    student_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    query VARCHAR(200) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX student_tutor_searches_student_created_idx
    ON student_tutor_searches (student_id, created_at DESC);

CREATE TABLE tutor_bookings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    student_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tutor_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scheduled_date DATE NOT NULL,
    scheduled_time VARCHAR(20) NOT NULL,
    mode VARCHAR(20) NOT NULL CHECK (mode IN ('home', 'online', 'group')),
    duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
    total_amount INTEGER NOT NULL CHECK (total_amount >= 0),
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'confirmed', 'cancelled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX tutor_bookings_student_date_idx ON tutor_bookings (student_id, scheduled_date);
CREATE INDEX tutor_bookings_tutor_date_idx ON tutor_bookings (tutor_id, scheduled_date);
