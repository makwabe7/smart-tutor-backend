-- Teachers registered before profile setup should still be discoverable in the
-- live tutor directory and able to resume editing their profile.
INSERT INTO teacher_profiles (user_id)
SELECT u.id
FROM users u
WHERE u.role = 'teacher'
ON CONFLICT (user_id) DO NOTHING;
