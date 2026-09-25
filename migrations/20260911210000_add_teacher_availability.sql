ALTER TABLE teacher_profiles
    ADD COLUMN can_travel BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN travel_radius_km INTEGER,
    ADD CONSTRAINT teacher_profiles_travel_radius_check CHECK (travel_radius_km IS NULL OR travel_radius_km BETWEEN 1 AND 100);

CREATE TABLE teacher_availability (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    teacher_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    weekday SMALLINT NOT NULL CHECK (weekday BETWEEN 0 AND 6),
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT teacher_availability_time_check CHECK (end_time > start_time),
    CONSTRAINT teacher_availability_unique UNIQUE (teacher_id, weekday)
);

CREATE INDEX teacher_availability_teacher_idx ON teacher_availability (teacher_id, weekday);
