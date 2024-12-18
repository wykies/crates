-- Notes on converting from MySql to Postgres (Left side is regex and right side is replacement, first is empty)
-- ` ->
-- int\(\d+\) -> int
-- int UNSIGNED -> bigint
-- engine.+; -> ;
-- tinyint\(\d+\) -> boolean
-- smallint\(\d+\) -> smallint
-- \buser\b -> users
-- comment '.+, -> ,
-- binary -> varchar
-- Timestamp -> unix_timestamp
--
-- Primary keys were done manually but could have been done see chat migration
-- Indices were also not created but could have been, however the syntax was pretty different (see chat migration).
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
  LockedEditing boolean NOT NULL DEFAULT false
);
-- --------------------------------------------------------
--
-- Table structure for table users
--

CREATE TABLE users (
  UserName varchar(16) NOT NULL PRIMARY KEY,
  Password varchar(64) NOT NULL,
  salt varchar(64) NOT NULL,
  LockedEditing boolean NOT NULL DEFAULT false,
  ForcePassChange boolean NOT NULL DEFAULT true,
  DisplayName varchar(30) NOT NULL UNIQUE,
  AssignedRole int DEFAULT NULL,
  PassChangeDate date NOT NULL,
  Enabled boolean NOT NULL DEFAULT true,
  LockedOut boolean NOT NULL DEFAULT false,
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