-- f
-- Create Seed Role
INSERT INTO `roles` (
        `RoleID`,
        `Name`,
        `Description`,
        `Permissions`,
        `LockedEditing`
    )
VALUES (
        NULL,
        'SeedAdmin',
        'Full Permissions',
        '11111111111111111111111111111111111',
        '0'
    );
-- Create Seed User
INSERT INTO `user` (
        `UserName`,
        `Password`,
        `password_hash`,
        `salt`,
        `DisplayName`,
        `AssignedRole`,
        `PassChangeDate`,
        `Enabled`
    )
VALUES (
        'seed_admin',
        '',
        '$argon2id$v=19$m=15000,t=2,p=1$MKnXfAG4x97WMzfWuOjs1g$MWvmzgFNfj8lneYHgghXuzXCpX+fs1NbVcWr2ieev8M',
        '',
        'Seed Admin User',
        LAST_INSERT_ID(),
        CURRENT_DATE(),
        1
    );
-- Create Seed Branch
INSERT INTO `branch` (`BranchID`, `BranchName`, `BranchAddress`)
VALUES (NULL, 'Seed Branch', '');