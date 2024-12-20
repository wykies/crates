-- f
-- Create Seed Role
INSERT INTO roles (
        role_name,
        role_description,
        permissions,
        locked_editing
    )
VALUES (
        'SeedAdmin',
        'Full Permissions',
        '11111111111111111111111111111111111',
        false
    );
-- Create Seed users
INSERT INTO users (
        user_name,
        password_hash,
        display_name,
        assigned_role,
        pass_change_date,
        is_enabled
    )
VALUES (
        'seed_admin',
        '$argon2id$v=19$m=15000,t=2,p=1$MKnXfAG4x97WMzfWuOjs1g$MWvmzgFNfj8lneYHgghXuzXCpX+fs1NbVcWr2ieev8M',
        'Seed Admin User',
        LASTVAL(),
        CURRENT_DATE,
        true
    );
-- Create Seed Branch
INSERT INTO branch (branch_name, branch_address)
VALUES ('Seed Branch', '');