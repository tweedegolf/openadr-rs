-- Add migration script here

create table program
(
    id                     text        not null
        constraint program_pk
            primary key,
    created_date_time      timestamptz not null,
    modification_date_time timestamptz not null,

    program_name           text        not null,
    program_long_name      text,
    retailer_name          text,
    retailer_long_name     text,
    program_type           text,
    country                text,
    principal_subdivision  text,
    -- deliberately omitted: time_zone_offset
    interval_period        jsonb,
    program_descriptions   jsonb,
    binding_events         boolean,
    local_price            boolean,
    payload_descriptors    jsonb
);

create unique index program_program_name_uindex
    on program (program_name);

create table event
(
    id                     text        not null
        constraint event_pk
            primary key,
    created_date_time      timestamptz not null,
    modification_date_time timestamptz not null,

    program_id             text        not null references program (id),
    event_name             text,
    priority               bigint,
    report_descriptors     jsonb,
    payload_descriptors    jsonb,
    interval_period        jsonb,
    intervals              jsonb       not null
);

create unique index event_event_name_uindex
    on event (event_name);


create table report
(
    id                     text        not null
        constraint report_pk
            primary key,
    created_date_time      timestamptz not null,
    modification_date_time timestamptz not null,

    program_id             text        not null references program (id),
    event_id               text        not null references event (id),
    client_name            text        not null,
    report_name            text,
    payload_descriptors    jsonb,
    resources              jsonb
);

create unique index report_report_name_uindex
    on report (report_name);

create table target
(
    id    bigserial primary key,
    label text not null,
    value text not null
);

create table program_target
(
    program_id text references program (id),
    target_id  bigserial references target (id)
);

create table event_target
(
    event_id  text references event (id),
    target_id bigserial references target (id)
);

CREATE TYPE target_tuple AS
    (
    label text,
    value text
    );
