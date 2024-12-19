-- --------------------------------------------------------
--
-- Table structure for table chat
--

CREATE TABLE chat (
    chat_id serial NOT NULL,
    author varchar(16) NOT NULL,
    unix_timestamp bigint NOT NULL,
    content varchar(255) NOT NULL
);
--
-- Indexes for table chat
--
ALTER TABLE chat
ADD PRIMARY KEY (chat_id);
CREATE INDEX ON chat (unix_timestamp);
--
-- Constraints for table chat
--
ALTER TABLE chat
ADD CONSTRAINT chat_ibfk_1 FOREIGN KEY (author) REFERENCES users (user_name);