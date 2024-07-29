INSERT INTO program (id, created_date_time, modification_date_time, program_name)
VALUES ('program-1', now(), now(), 'Program Eins'),
       ('program-2', now(), now(), 'Program Zwei');

WITH ins_event AS (INSERT INTO event (id, created_date_time, modification_date_time, program_id, event_name, priority,
                                      report_descriptors, payload_descriptors, interval_period, intervals)
    VALUES (gen_random_uuid(),
            now(),
            now(),
            'program-1',
            'event-5-name',
            '4',
            null,
            null,
            '{
              "start": "2023-06-15T09:30:00+00:00",
              "duration": "P0Y0M0DT1H0M0S",
              "randomizeStart": "P0Y0M0DT1H0M0S"
            }'::jsonb,
            '[
              {
                "id": 3,
                "payloads": [
                  {
                    "type": "PRICE",
                    "values": [
                      0.17
                    ]
                  }
                ],
                "intervalPeriod": {
                  "start": "2023-06-15T09:30:00+00:00",
                  "duration": "P0Y0M0DT1H0M0S",
                  "randomizeStart": "P0Y0M0DT1H0M0S"
                }
              }
            ]'::jsonb) RETURNING event.id),
     ins_target AS (INSERT INTO target (label, value)
         VALUES ('GROUP', 'group-3'), ('GROUP', 'group-4') ON CONFLICT DO NOTHING RETURNING id)
INSERT
INTO event_target (event_id, target_id)
SELECT ins_event.id, ins_target.id
FROM ins_target,
     ins_event
ON CONFLICT DO NOTHING;

WITH ins_event AS (INSERT INTO event (id, created_date_time, modification_date_time, program_id, event_name, priority,
                                      report_descriptors, payload_descriptors, interval_period, intervals)
    VALUES (gen_random_uuid(),
            now(),
            now(),
            'program-2',
            'event-2-name',
            null,
            null,
            null,
            null,
            '[
              {
                "id": 3,
                "payloads": [
                  {
                    "type": "SOME_PAYLOAD",
                    "values": [
                      "value"
                    ]
                  }
                ]
              }
            ]'::jsonb) RETURNING event.id),
     ins_target AS (INSERT INTO target (label, value)
         VALUES ('SOME_TARGET', 'target-1'), ('GROUP', 'group-4') ON CONFLICT DO NOTHING RETURNING id)
INSERT
INTO event_target (event_id, target_id)
SELECT ins_event.id, ins_target.id
FROM ins_target,
     ins_event
ON CONFLICT DO NOTHING;
