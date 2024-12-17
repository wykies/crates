-- f
-- Create Seed Role
INSERT INTO roles (
        Name,
        Description,
        Permissions,
        LockedEditing
    )
VALUES (
        'SeedAdmin',
        'Full Permissions',
        '11111111111111111111111111111111111',
        '0'
    );
-- Create Seed users
INSERT INTO users (
        UserName,
        Password,
        password_hash,
        salt,
        DisplayName,
        AssignedRole,
        PassChangeDate,
        Enabled
    )
VALUES (
        'seed_admin',
        '',
        '$argon2id$v=19$m=15000,t=2,p=1$MKnXfAG4x97WMzfWuOjs1g$MWvmzgFNfj8lneYHgghXuzXCpX+fs1NbVcWr2ieev8M',
        '',
        'Seed Admin users',
        LASTVAL(),
        CURRENT_DATE,
        1
    );
-- Create Seed Branch
INSERT INTO branch (BranchName, BranchAddress)
VALUES ('Seed Branch', '');