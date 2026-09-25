CREATE TABLE parent_students (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    parent_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    student_id UUID REFERENCES users(id) ON DELETE CASCADE,
    child_name VARCHAR(200) NOT NULL,
    grade_level VARCHAR(50),
    relationship VARCHAR(50) NOT NULL DEFAULT 'Parent',
    status VARCHAR(20) NOT NULL CHECK (status IN ('managed', 'pending', 'linked')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT parent_student_distinct CHECK (parent_id <> student_id)
);

CREATE UNIQUE INDEX parent_students_unique_link_idx
    ON parent_students (parent_id, student_id) WHERE student_id IS NOT NULL;
CREATE INDEX parent_students_parent_idx ON parent_students (parent_id);
CREATE INDEX parent_students_student_idx ON parent_students (student_id) WHERE student_id IS NOT NULL;
