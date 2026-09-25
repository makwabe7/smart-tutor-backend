CREATE TABLE tuition_centres (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id UUID REFERENCES users(id) ON DELETE SET NULL,
    slug VARCHAR(180) UNIQUE NOT NULL,
    name VARCHAR(180) NOT NULL,
    tagline VARCHAR(240),
    county VARCHAR(100) NOT NULL DEFAULT 'Nairobi',
    estate VARCHAR(100) NOT NULL,
    address TEXT,
    description TEXT,
    subjects TEXT[] NOT NULL DEFAULT '{}',
    grade_levels TEXT[] NOT NULL DEFAULT '{}',
    monthly_fee INTEGER NOT NULL DEFAULT 0 CHECK (monthly_fee >= 0),
    student_count INTEGER NOT NULL DEFAULT 0 CHECK (student_count >= 0),
    rating NUMERIC(2, 1) NOT NULL DEFAULT 0 CHECK (rating >= 0 AND rating <= 5),
    review_count INTEGER NOT NULL DEFAULT 0 CHECK (review_count >= 0),
    image_url TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX tuition_centres_active_estate_idx ON tuition_centres (estate) WHERE is_active;
CREATE INDEX tuition_centres_active_name_idx ON tuition_centres (name) WHERE is_active;
