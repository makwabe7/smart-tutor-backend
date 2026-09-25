-- Development/demo tutor profiles for the public Top Tutors section.
-- Each email is deterministic, making this seed safe to re-run in a fresh environment.
WITH seed_users AS (
    INSERT INTO users (email, phone, password_hash, first_name, last_name, role, is_verified, is_active)
    VALUES
      ('demo.mercy.wanjiru@soma.test', '+254700000101', 'demo-profile-not-for-login', 'Mercy', 'Wanjiru', 'teacher', TRUE, TRUE),
      ('demo.brian.otieno@soma.test', '+254700000102', 'demo-profile-not-for-login', 'Brian', 'Otieno', 'teacher', TRUE, TRUE),
      ('demo.aisha.hassan@soma.test', '+254700000103', 'demo-profile-not-for-login', 'Aisha', 'Hassan', 'teacher', TRUE, TRUE),
      ('demo.daniel.mwangi@soma.test', '+254700000104', 'demo-profile-not-for-login', 'Daniel', 'Mwangi', 'teacher', TRUE, TRUE)
    ON CONFLICT (email) DO UPDATE SET is_active = TRUE, is_verified = TRUE
    RETURNING id, email
)
INSERT INTO teacher_profiles (user_id, county, estate, education_level, subjects, grades, years_experience, hourly_rate, bio, photo_url, intro_video_url, is_verified, can_travel, travel_radius_km)
SELECT seed_users.id,
       values_data.county, values_data.estate, values_data.education_level, values_data.subjects, values_data.grades,
       values_data.years_experience, values_data.hourly_rate, values_data.bio, values_data.photo_url, values_data.intro_video_url,
       TRUE, values_data.can_travel, values_data.travel_radius_km
FROM seed_users
JOIN (VALUES
  ('demo.mercy.wanjiru@soma.test', 'Nairobi', 'Kasarani', 'B.Ed Mathematics', ARRAY['Mathematics', 'Chemistry']::TEXT[], ARRAY['Grade 7–8', 'Form 1–4']::TEXT[], 9, 850, 'Patient, exam-focused mathematics tutor with a practical approach to KCSE revision.', 'https://images.unsplash.com/photo-1494790108377-be9c29b29330?auto=format&fit=crop&w=800&q=80', NULL::TEXT, TRUE, 12),
  ('demo.brian.otieno@soma.test', 'Nairobi', 'Roysambu', 'B.Sc Physics', ARRAY['Physics', 'Mathematics']::TEXT[], ARRAY['Form 1–4']::TEXT[], 7, 900, 'Physics and maths educator helping learners build confidence through weekly practice.', 'https://images.unsplash.com/photo-1500648767791-00dcc994a43e?auto=format&fit=crop&w=800&q=80', NULL::TEXT, TRUE, 10),
  ('demo.aisha.hassan@soma.test', 'Nairobi', 'Mirema', 'B.A Education', ARRAY['English', 'Literature', 'Kiswahili']::TEXT[], ARRAY['Grade 4–8', 'Form 1–4']::TEXT[], 11, 800, 'Language specialist supporting stronger reading, writing, and composition skills.', 'https://images.unsplash.com/photo-1534528741775-53994a69daeb?auto=format&fit=crop&w=800&q=80', NULL::TEXT, TRUE, 15),
  ('demo.daniel.mwangi@soma.test', 'Kiambu', 'Ruiru', 'B.Sc Computer Science', ARRAY['Computer Studies', 'Mathematics']::TEXT[], ARRAY['Form 1–4']::TEXT[], 6, 950, 'STEM tutor who makes technical concepts accessible with hands-on learning.', 'https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?auto=format&fit=crop&w=800&q=80', NULL::TEXT, FALSE, NULL)
) AS values_data(email, county, estate, education_level, subjects, grades, years_experience, hourly_rate, bio, photo_url, intro_video_url, can_travel, travel_radius_km)
  ON values_data.email = seed_users.email
ON CONFLICT (user_id) DO UPDATE SET
  county = EXCLUDED.county, estate = EXCLUDED.estate, education_level = EXCLUDED.education_level,
  subjects = EXCLUDED.subjects, grades = EXCLUDED.grades, years_experience = EXCLUDED.years_experience,
  hourly_rate = EXCLUDED.hourly_rate, bio = EXCLUDED.bio, photo_url = EXCLUDED.photo_url,
  intro_video_url = EXCLUDED.intro_video_url, is_verified = TRUE, can_travel = EXCLUDED.can_travel,
  travel_radius_km = EXCLUDED.travel_radius_km, updated_at = NOW();
