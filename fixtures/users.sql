INSERT INTO "user" (id)
VALUES ('admin');

INSERT INTO user_roles (user_id, role)
VALUES ('admin', '{
  "role": "AnyBusiness"
}'::jsonb);
INSERT INTO user_roles (user_id, role)
VALUES ('admin', '{
  "role": "UserManager"
}'::jsonb);

INSERT INTO user_credentials (user_id, client_id, client_secret)
VALUES ('admin', 'admin', 'admin');

INSERT INTO "user" (id)
VALUES ('user-1');

INSERT INTO user_roles (user_id, role)
VALUES ('user-1', '{
  "role": "VEN",
  "id": "ven-1"
}'::jsonb);
