-- Additional demo tutors for browsing and the "View more tutors" flow.
WITH seed_users AS (
    INSERT INTO users (email, phone, password_hash, first_name, last_name, role, is_verified, is_active)
    VALUES
      ('demo.grace.nyambura@soma.test', '+254700000105', 'demo-profile-not-for-login', 'Grace', 'Nyambura', 'teacher', TRUE, TRUE),
      ('demo.kevin.kiptoo@soma.test', '+254700000106', 'demo-profile-not-for-login', 'Kevin', 'Kiptoo', 'teacher', TRUE, TRUE),
      ('demo.faith.achieng@soma.test', '+254700000107', 'demo-profile-not-for-login', 'Faith', 'Achieng', 'teacher', TRUE, TRUE),
      ('demo.joseph.kamau@soma.test', '+254700000108', 'demo-profile-not-for-login', 'Joseph', 'Kamau', 'teacher', TRUE, TRUE)
    ON CONFLICT (email) DO UPDATE SET is_active = TRUE, is_verified = TRUE
    RETURNING id, email
)
INSERT INTO teacher_profiles (user_id, county, estate, education_level, subjects, grades, years_experience, hourly_rate, bio, photo_url, is_verified, can_travel, travel_radius_km)
SELECT seed_users.id, values_data.county, values_data.estate, values_data.education_level, values_data.subjects, values_data.grades,
       values_data.years_experience, values_data.hourly_rate, values_data.bio, values_data.photo_url, TRUE, values_data.can_travel, values_data.travel_radius_km
FROM seed_users
JOIN (VALUES
  ('demo.grace.nyambura@soma.test', 'Nairobi', 'Kilimani', 'B.Ed Sciences', ARRAY['Biology', 'Chemistry']::TEXT[], ARRAY['Form 1–4']::TEXT[], 8, 880, 'Science tutor focused on clear explanations and practical revision techniques.', 'https://images.unsplash.com/photo-1531123897727-8f129e1688ce?auto=format&fit=crop&w=800&q=80', TRUE, 8),
  ('demo.kevin.kiptoo@soma.test', 'Nairobi', 'Westlands', 'B.Com Finance', ARRAY['Business Studies', 'Mathematics']::TEXT[], ARRAY['Form 1–4']::TEXT[], 5, 780, 'Business studies tutor who connects classroom learning to everyday examples.', 'https://images.unsplash.com/photo-1506794778202-cad84cf45f1d?auto=format&fit=crop&w=800&q=80', TRUE, 10),
  ('demo.faith.achieng@soma.test', 'Nairobi', 'South B', 'B.A Kiswahili', ARRAY['Kiswahili', 'English']::TEXT[], ARRAY['Grade 4–8', 'Form 1–4']::TEXT[], 10, 820, 'Dedicated language coach for confident speaking, composition, and comprehension.', 'https://images.unsplash.com/photo-1544005313-94ddf0286df2?auto=format&fit=crop&w=800&q=80', TRUE, 12),
  ('demo.joseph.kamau@soma.test', 'Kiambu', 'Thika', 'B.Sc Mathematics', ARRAY['Mathematics', 'Computer Studies']::TEXT[], ARRAY['Grade 7–8', 'Form 1–4']::TEXT[], 4, 700, 'Supportive STEM tutor for learners preparing for school assessments and exams.', 'https://images.unsplash.com/photo-1519085360753-af0119f7cbe7?auto=format&fit=crop&w=800&q=80', FALSE, NULL)
) AS values_data(email, county, estate, education_level, subjects, grades, years_experience, hourly_rate, bio, photo_url, can_travel, travel_radius_km)
  ON values_data.email = seed_users.email
ON CONFLICT (user_id) DO UPDATE SET
  county = EXCLUDED.county, estate = EXCLUDED.estate, education_level = EXCLUDED.education_level,
  subjects = EXCLUDED.subjects, grades = EXCLUDED.grades, years_experience = EXCLUDED.years_experience,
  hourly_rate = EXCLUDED.hourly_rate, bio = EXCLUDED.bio, photo_url = EXCLUDED.photo_url,
  is_verified = TRUE, can_travel = EXCLUDED.can_travel, travel_radius_km = EXCLUDED.travel_radius_km, updated_at = NOW();
