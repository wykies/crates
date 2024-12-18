-- --------------------------------------------------------
--
-- Table structure for table `chat`
--

CREATE TABLE `chat` (
    `ChatID` int(11) NOT NULL,
    `Author` varchar(16) NOT NULL,
    `Timestamp` INT(11) UNSIGNED NOT NULL,
    `Content` BINARY(255) NOT NULL
) ENGINE = InnoDB DEFAULT CHARSET = latin1;
--
-- Indexes for table `chat`
--
ALTER TABLE `chat`
ADD PRIMARY KEY (`ChatID`),
    ADD KEY `Timestamp` (`Timestamp`);
--
-- AUTO_INCREMENT for table `chat`
--
ALTER TABLE `chat`
MODIFY `ChatID` int(11) NOT NULL AUTO_INCREMENT;
--
-- Constraints for table `chat`
--
ALTER TABLE `chat`
ADD CONSTRAINT `chat_ibfk_1` FOREIGN KEY (`Author`) REFERENCES `user` (`UserName`);