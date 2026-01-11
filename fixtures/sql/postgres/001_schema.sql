drop schema if exists app cascade;
create schema app;

create type app.status as enum ('pending', 'active', 'disabled');

create table app.users (
  id bigint generated always as identity primary key,
  email text not null,
  full_name text not null,
  age integer not null default 0,
  status app.status not null default 'pending',
  created_at timestamptz not null default now(),
  bio text,
  constraint age_non_negative check (age >= 0),
  constraint users_email_key unique (email)
);

create index idx_users_status on app.users(status);

create table app.orders (
  id bigint generated always as identity primary key,
  user_id bigint not null,
  total numeric(10,2) not null default 0,
  status app.status not null default 'pending',
  constraint orders_user_fk foreign key (user_id) references app.users(id) on delete cascade on update no action,
  constraint orders_user_status_unique unique (user_id, status) deferrable initially deferred
);

create index idx_orders_user on app.orders(user_id);

create view app.active_users as
  select id, email, full_name
  from app.users
  where status = 'active';
