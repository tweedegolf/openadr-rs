INSERT INTO "user" (id, reference, description, created, modified)
VALUES ('admin', 'admin-ref', null, now(), now());

INSERT INTO user_business VALUES ('admin', NULL);

INSERT INTO user_credentials (user_id, client_id, client_secret)
VALUES ('admin', 'admin', '$argon2id$v=19$m=16,t=2,p=1$QmtwZnBPVnlIYkJTWUtHZg$lMxF0N+CeRa99UmzMaUKeg'); -- secret: admin

INSERT INTO "user" (id, reference, description, created, modified)
VALUES ('user-1', 'user-1-ref', null, now(), now());