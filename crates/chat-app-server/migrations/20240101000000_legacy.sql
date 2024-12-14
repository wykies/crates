SET SQL_MODE = "NO_AUTO_VALUE_ON_ZERO";
START TRANSACTION;
SET time_zone = "+00:00";
/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */
;
/*!40101 SET @OLD_CHARACTER_SET_RESULTS=@@CHARACTER_SET_RESULTS */
;
/*!40101 SET @OLD_COLLATION_CONNECTION=@@COLLATION_CONNECTION */
;
/*!40101 SET NAMES utf8mb4 */
;
--
-- Table structure for table `branch`
--

CREATE TABLE `branch` (
  `BranchID` int(11) NOT NULL,
  `BranchName` varchar(30) NOT NULL,
  `BranchAddress` varchar(200) NOT NULL
) ENGINE = InnoDB DEFAULT CHARSET = latin1;
-- --------------------------------------------------------
--
-- Table structure for table `hostbranch`
--

CREATE TABLE `hostbranch` (
  `hostname` varchar(50) NOT NULL,
  `AssignedBranch` int(11) NOT NULL
) ENGINE = InnoDB DEFAULT CHARSET = latin1;
-- --------------------------------------------------------
--
-- Table structure for table `roles`
--

CREATE TABLE `roles` (
  `RoleID` int(11) NOT NULL,
  `Name` varchar(16) NOT NULL,
  `Description` varchar(50) NOT NULL DEFAULT '',
  `Permissions` varchar(256) NOT NULL,
  `LockedEditing` tinyint(1) NOT NULL DEFAULT '0'
) ENGINE = InnoDB DEFAULT CHARSET = latin1;
-- --------------------------------------------------------
--
-- Table structure for table `user`
--

CREATE TABLE `user` (
  `UserName` varchar(16) NOT NULL,
  `Password` binary(64) NOT NULL,
  `salt` binary(64) NOT NULL,
  `LockedEditing` tinyint(1) NOT NULL DEFAULT '0',
  `ForcePassChange` tinyint(1) NOT NULL DEFAULT '1',
  `DisplayName` varchar(30) NOT NULL,
  `AssignedRole` int(11) DEFAULT NULL,
  `PassChangeDate` date NOT NULL,
  `Enabled` tinyint(1) NOT NULL DEFAULT '1',
  `LockedOut` tinyint(1) NOT NULL DEFAULT '0',
  `FailedAttempts` tinyint(4) NOT NULL DEFAULT '0' COMMENT 'Number of Failed Attempts Since Last Logon',
  `AsycudaName` varchar(50) DEFAULT NULL
) ENGINE = InnoDB DEFAULT CHARSET = latin1;
-- --------------------------------------------------------
--
-- Table structure for table `version_info`
--

CREATE TABLE `version_info` (
  `dbversion` smallint(6) NOT NULL,
  `MinExeVersion` varchar(15) NOT NULL
) ENGINE = InnoDB DEFAULT CHARSET = latin1 COMMENT = 'Stores the DB version for compatibility reason';
--
-- Indexes for dumped tables
--

--
-- Indexes for table `branch`
--
ALTER TABLE `branch`
ADD PRIMARY KEY (`BranchID`),
  ADD UNIQUE KEY `BranchName` (`BranchName`);
--
-- Indexes for table `hostbranch`
--
ALTER TABLE `hostbranch`
ADD PRIMARY KEY (`hostname`),
  ADD KEY `AssignedBranch` (`AssignedBranch`);
--
-- Indexes for table `roles`
--
ALTER TABLE `roles`
ADD PRIMARY KEY (`RoleID`),
  ADD UNIQUE KEY `Name` (`Name`);
--
-- Indexes for table `user`
--
ALTER TABLE `user`
ADD PRIMARY KEY (`UserName`),
  ADD UNIQUE KEY `DisplayName` (`DisplayName`),
  ADD KEY `AssignedRole` (`AssignedRole`);
--
-- Indexes for table `version_info`
--
ALTER TABLE `version_info`
ADD PRIMARY KEY (`dbversion`);
--
-- AUTO_INCREMENT for dumped tables
--

--
-- AUTO_INCREMENT for table `branch`
--
ALTER TABLE `branch`
MODIFY `BranchID` int(11) NOT NULL AUTO_INCREMENT;
--
-- AUTO_INCREMENT for table `roles`
--
ALTER TABLE `roles`
MODIFY `RoleID` int(11) NOT NULL AUTO_INCREMENT;
--
-- Constraints for dumped tables
--

--
-- Constraints for table `hostbranch`
--
ALTER TABLE `hostbranch`
ADD CONSTRAINT `hostbranch_ibfk_1` FOREIGN KEY (`AssignedBranch`) REFERENCES `branch` (`BranchID`);
--
-- Constraints for table `user`
--
ALTER TABLE `user`
ADD CONSTRAINT `user_ibfk_1` FOREIGN KEY (`AssignedRole`) REFERENCES `roles` (`RoleID`);
COMMIT;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */
;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */
;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */
;