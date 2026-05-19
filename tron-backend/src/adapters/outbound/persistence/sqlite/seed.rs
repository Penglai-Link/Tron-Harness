use crate::{
    domain::user::{PhoneNumber, User, UserSourceType, UserStatus},
    shared::id::SnowflakeId,
};

const AUTH_OPERATOR_PASSWORD: &str = "TronAuthPass123!";

pub(super) fn seeded_users() -> Vec<User> {
    let mut users = vec![
        seeded_user(
            734_102_998_871_252_992,
            "USR-0001",
            "alex.johnson",
            "Alex",
            "Johnson",
            "Alex Johnson",
            "alex.johnson@tron.ai",
            UserStatus::Active,
            vec!["DEVELOPER", "VIEWER"],
            vec!["PLATFORM", "DATA"],
        ),
        seeded_user(
            734_103_002_743_185_408,
            "USR-0002",
            "maya.singh",
            "Maya",
            "Singh",
            "Maya Singh",
            "maya.singh@tron.ai",
            UserStatus::Active,
            vec!["VIEWER", "DEVELOPER"],
            vec!["ENTERPRISE", "DATA"],
        ),
        seeded_user(
            734_103_002_743_185_409,
            "USR-0007",
            "sam.viewer",
            "Sam",
            "Viewer",
            "Sam Viewer",
            "sam.viewer@tron.ai",
            UserStatus::Active,
            vec!["VIEWER"],
            vec!["SUPPORT", "PLATFORM"],
        ),
        seeded_user(
            734_103_002_743_185_410,
            "USR-0009",
            "archived.restore",
            "Archived",
            "Restore",
            "Archived Restore",
            "archived.restore@tron.ai",
            UserStatus::Archived,
            vec!["DEVELOPER", "VIEWER"],
            vec!["DATA", "SUPPORT"],
        ),
        seeded_user(
            734_103_002_743_185_411,
            "USR-1001",
            "reset.target",
            "Reset",
            "Target",
            "Reset Target",
            "reset.target@tron.ai",
            UserStatus::Active,
            vec!["ADMIN", "DEVELOPER"],
            vec!["PLATFORM", "ENTERPRISE"],
        ),
        seeded_user(
            9_007_199_254_740_993,
            "USR-LARGE-ID",
            "large.id",
            "Large",
            "Identifier",
            "Large Identifier",
            "large.id@tron.ai",
            UserStatus::Active,
            vec!["WORKSPACE_ADMIN", "DEVELOPER"],
            vec!["ENTERPRISE", "PLATFORM"],
        ),
        seeded_user(
            734_103_002_743_185_412,
            "USR-ARCHIVED",
            "archived.only",
            "Archived",
            "Only",
            "Archived Only",
            "archived.only@tron.ai",
            UserStatus::Archived,
            vec!["VIEWER"],
            vec!["SUPPORT", "DATA"],
        ),
    ];

    users.push(seeded_auth_user(
        734_103_002_743_185_500,
        "USR-AUTH-0001",
        "auth.operator",
        "Auth",
        "Operator",
        "Auth Operator",
        "auth.operator@tron.ai",
        UserStatus::Active,
    ));
    users.push(seeded_auth_user(
        734_103_002_743_185_501,
        "USR-AUTH-ARCHIVED",
        "archived.auth",
        "Archived",
        "Auth",
        "Archived Auth",
        "archived.auth@tron.ai",
        UserStatus::Archived,
    ));

    users
}

fn seeded_user(
    id: i64,
    user_code: &str,
    username: &str,
    first_name: &str,
    last_name: &str,
    display_name: &str,
    email: &str,
    status: UserStatus,
    role_codes: Vec<&str>,
    space_codes: Vec<&str>,
) -> User {
    User {
        id: SnowflakeId::new(id),
        user_code: user_code.into(),
        source_type: UserSourceType::Internal,
        username: username.into(),
        first_name: first_name.into(),
        last_name: last_name.into(),
        display_name: display_name.into(),
        email: email.into(),
        phone: Some(PhoneNumber {
            country_code: "+1".into(),
            number: "5550100".into(),
        }),
        avatar_url: None,
        locale: "en-US".into(),
        timezone: "America/Los_Angeles".into(),
        status,
        role_codes: role_codes.into_iter().map(str::to_owned).collect(),
        space_codes: space_codes.into_iter().map(str::to_owned).collect(),
        password_hash: "$2b$04$wzYH5I9Aw4f3ZV5h9tczhOJtT5QxQfyeZDJmW5hE4d1tij8nS0V9G".into(),
        password_reset_required: false,
        last_active_at: Some("2026-05-11T08:30:00Z".into()),
        created_at: "2026-05-11T08:00:00Z".into(),
        updated_at: "2026-05-11T08:00:00Z".into(),
        archived_at: (status == UserStatus::Archived).then(|| "2026-05-11T08:00:00Z".into()),
    }
}

fn seeded_auth_user(
    id: i64,
    user_code: &str,
    username: &str,
    first_name: &str,
    last_name: &str,
    display_name: &str,
    email: &str,
    status: UserStatus,
) -> User {
    let mut user = seeded_user(
        id,
        user_code,
        username,
        first_name,
        last_name,
        display_name,
        email,
        status,
        vec!["ADMIN", "DEVELOPER"],
        vec!["PLATFORM", "DATA"],
    );
    user.password_hash =
        bcrypt::hash(AUTH_OPERATOR_PASSWORD, 4).expect("auth seed password hash should be generated");
    user.password_reset_required = false;
    user
}
