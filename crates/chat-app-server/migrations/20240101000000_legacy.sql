START TRANSACTION;
--
-- Table structure for table branch
--

CREATE TABLE branch (
  BranchID serial NOT NULL PRIMARY KEY,
  BranchName varchar(30) NOT NULL UNIQUE,
  BranchAddress varchar(200) NOT NULL
);
-- --------------------------------------------------------
--
-- Table structure for table hostbranch
--

CREATE TABLE hostbranch (
  hostname varchar(50) NOT NULL PRIMARY KEY,
  AssignedBranch int NOT NULL
);
-- --------------------------------------------------------
--
-- Table structure for table roles
--

CREATE TABLE roles (
  RoleID serial NOT NULL PRIMARY KEY,
  Name varchar(16) NOT NULL UNIQUE,
  Description varchar(50) NOT NULL DEFAULT '',
  Permissions varchar(256) NOT NULL,
  LockedEditing smallint NOT NULL DEFAULT '0'
);
-- --------------------------------------------------------
--
-- Table structure for table users
--

CREATE TABLE users (
  UserName varchar(16) NOT NULL PRIMARY KEY,
  Password varchar(64) NOT NULL,
  salt varchar(64) NOT NULL,
  LockedEditing smallint NOT NULL DEFAULT '0',
  ForcePassChange smallint NOT NULL DEFAULT '1',
  DisplayName varchar(30) NOT NULL UNIQUE,
  AssignedRole int DEFAULT NULL,
  PassChangeDate date NOT NULL,
  Enabled smallint NOT NULL DEFAULT '1',
  LockedOut smallint NOT NULL DEFAULT '0',
  FailedAttempts smallint NOT NULL DEFAULT '0'
);
--
-- Constraints for dumped tables
--

--
-- Constraints for table hostbranch
--
ALTER TABLE hostbranch
ADD CONSTRAINT hostbranch_ibfk_1 FOREIGN KEY (AssignedBranch) REFERENCES branch (BranchID);
--
-- Constraints for table users
--
ALTER TABLE users
ADD CONSTRAINT user_ibfk_1 FOREIGN KEY (AssignedRole) REFERENCES roles (RoleID);
COMMIT;