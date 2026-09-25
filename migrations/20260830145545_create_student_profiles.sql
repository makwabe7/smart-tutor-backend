-- Add migration script here
CREATE TABLE student_profiles(
    id UUID PRIMARY KEY  DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL UNIQUE,
    date_of_birth DATE,
    gender VARCHAR(20),
    school_name VARCHAR(255),
    grade_level VARCHAR(50),
    county VARCHAR(100),
    bio TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_student_user
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE





);