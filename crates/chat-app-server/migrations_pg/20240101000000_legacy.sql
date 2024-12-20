-- Notes on converting from MySql to Postgres (Left side is regex and right side is replacement, first is empty)
-- ` ->
-- int\(\d+\) -> int
-- int UNSIGNED -> bigint
-- engine.+; -> ;
-- tinyint\(\d+\) -> boolean
-- smallint\(\d+\) -> smallint
-- comment '.+, -> ,
-- binary -> varchar
-- Timestamp -> unix_timestamp
-- Table and 
-- Reserved (Special) words avoidance
-- user -> users # NB: The seed user has the word user in their name
-- name -> role_name
-- description -> role_description
-- enabled -> is_enabled
--
-- Primary keys were done manually but could have been done see chat migration
-- Indices were also not created but could have been, however the syntax was pretty different (see chat migration).
--
-- Table structure for table branch
--

CREATE TABLE branch (
  branch_id serial NOT NULL PRIMARY KEY,
  branch_name varchar(30) NOT NULL UNIQUE,
  branch_address varchar(200) NOT NULL
);
-- --------------------------------------------------------
--
-- Table structure for table hostbranch
--

CREATE TABLE hostbranch (
  hostname varchar(50) NOT NULL PRIMARY KEY,
  assigned_branch int NOT NULL
);
-- --------------------------------------------------------
--
-- Table structure for table roles
--

CREATE TABLE roles (
  role_id serial NOT NULL PRIMARY KEY,
  role_name varchar(16) NOT NULL UNIQUE,
  role_description varchar(50) NOT NULL DEFAULT '',
  permissions varchar(256) NOT NULL,
  locked_editing boolean NOT NULL DEFAULT false
);
-- --------------------------------------------------------
--
-- Table structure for table users
--

CREATE TABLE users (
  user_name varchar(16) NOT NULL PRIMARY KEY,
  force_pass_change boolean NOT NULL DEFAULT true,
  display_name varchar(30) NOT NULL UNIQUE,
  assigned_role int DEFAULT NULL,
  pass_change_date date NOT NULL,
  is_enabled boolean NOT NULL DEFAULT true,
  locked_out boolean NOT NULL DEFAULT false,
  failed_attempts smallint NOT NULL DEFAULT '0'
);
--
-- Constraints for dumped tables
--

--
-- Constraints for table hostbranch
--
ALTER TABLE hostbranch
ADD CONSTRAINT hostbranch_ibfk_1 FOREIGN KEY (assigned_branch) REFERENCES branch (branch_id);
--
-- Constraints for table users
--
ALTER TABLE users
ADD CONSTRAINT user_ibfk_1 FOREIGN KEY (assigned_role) REFERENCES roles (role_id);