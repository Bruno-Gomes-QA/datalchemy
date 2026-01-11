insert into app.users (email, full_name, age, status, bio) values
  ('alice@example.com', 'Alice', 30, 'active', 'likes rust'),
  ('bob@example.com', 'Bob', 25, 'pending', null);

insert into app.orders (user_id, total, status) values
  (1, 99.95, 'active'),
  (2, 15.00, 'pending');
