-- Add migration script here
ALTER TABLE `user`
ADD `password_hash` TEXT NOT NULL
AFTER `Password`;