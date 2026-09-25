CREATE TABLE teacher_profiles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    county VARCHAR(100),
    estate VARCHAR(100),
    education_level VARCHAR(100),
    subjects TEXT[] NOT NULL DEFAULT '{}',
    grades TEXT[] NOT NULL DEFAULT '{}',
    years_experience INTEGER NOT NULL DEFAULT 0,
    hourly_rate INTEGER NOT NULL DEFAULT 0,
    bio TEXT,
    photo_url TEXT,
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT teacher_profiles_years_experience_check CHECK (years_experience >= 0),
    CONSTRAINT teacher_profiles_hourly_rate_check CHECK (hourly_rate >= 0)
);

CREATE INDEX teacher_profiles_estate_idx ON teacher_profiles (estate);
