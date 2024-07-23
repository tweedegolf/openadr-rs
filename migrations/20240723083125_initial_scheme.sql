-- Add migration script here

create table event
(
    id                     text        not null
        constraint event_pk
            primary key,
    created_date_time      timestamptz not null,
    modification_date_time timestamptz not null,
    program_id             text        not null, -- TODO add foreign key constraint
    event_name             text,
    priority               bigint,
    targets                jsonb,
    report_descriptors     jsonb,
    payload_descriptors    jsonb,
    interval_period        jsonb,
    intervals              jsonb       not null
);

create unique index event_event_name_uindex
    on public.event (event_name);