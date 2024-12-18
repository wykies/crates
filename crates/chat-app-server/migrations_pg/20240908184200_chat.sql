-- --------------------------------------------------------
--
-- Table structure for table chat
--

CREATE TABLE chat (
    ChatID serial NOT NULL,
    Author varchar(16) NOT NULL,
    unix_timestamp bigint NOT NULL,
    Content varchar(255) NOT NULL
);
--
-- Indexes for table chat
--
ALTER TABLE chat
ADD PRIMARY KEY (ChatID);
CREATE INDEX ON chat (unix_timestamp);
--
-- Constraints for table chat
--
ALTER TABLE chat
ADD CONSTRAINT chat_ibfk_1 FOREIGN KEY (Author) REFERENCES users (UserName);