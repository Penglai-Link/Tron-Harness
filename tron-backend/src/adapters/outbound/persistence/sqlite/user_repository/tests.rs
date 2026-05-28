use super::SqliteUserRepository;
use crate::{
    application::user::UserRepository,
    domain::{
        auth::OAuth2IdentityLink,
        user::{PhoneNumber, User, UserSourceType, UserStatus},
    },
    shared::id::SnowflakeId,
};

fn seed_user(repository: &mut SqliteUserRepository) -> User {
    let user = User {
        id: SnowflakeId::new(734_102_998_871_252_992),
        user_code: "USR-0001".into(),
        source_type: UserSourceType::Internal,
        username: "alex.johnson".into(),
        first_name: "Alex".into(),
        last_name: "Johnson".into(),
        display_name: "Alex Johnson".into(),
        email: "alex.johnson@tron.ai".into(),
        phone: Some(PhoneNumber {
            country_code: "+1".into(),
            number: "5550100".into(),
        }),
        avatar_url: None,
        locale: "en-US".into(),
        timezone: "America/Los_Angeles".into(),
        status: UserStatus::Active,
        role_codes: vec!["DEVELOPER".into(), "VIEWER".into()],
        space_codes: vec!["PLATFORM".into(), "DATA".into()],
        password_hash: "$2b$04$wzYH5I9Aw4f3ZV5h9tczhOJtT5QxQfyeZDJmW5hE4d1tij8nS0V9G".into(),
        password_reset_required: false,
        last_active_at: Some("2026-05-11T08:30:00Z".into()),
        created_at: "2026-05-11T08:00:00Z".into(),
        updated_at: "2026-05-11T08:00:00Z".into(),
        archived_at: None,
    };
    repository
        .insert_user(&user)
        .expect("test user fixture should seed");
    user
}

#[test]
fn get_user_derives_github_source_type_from_oauth_identity() {
    let mut repository = SqliteUserRepository::connect(":memory:")
        .expect("in-memory user repository should initialize");
    let seeded_user = seed_user(&mut repository);
    assert_eq!(seeded_user.source_type, UserSourceType::Internal);

    let link_id = repository
        .next_id()
        .expect("oauth identity id should be generated");
    repository
        .upsert_oauth_identity(&OAuth2IdentityLink {
            id: link_id,
            user_id: seeded_user.id.value(),
            user_code: seeded_user.user_code.clone(),
            provider_code: "github".into(),
            provider_subject: "github-usr-0001".into(),
            email: seeded_user.email.clone(),
            display_name: seeded_user.display_name.clone(),
            linked_at: 1_778_544_000_000,
            updated_at: 1_778_544_000_000,
        })
        .expect("oauth identity should be linked");

    let github_user = repository
        .get_user("USR-0001")
        .expect("seeded user should still exist");
    assert_eq!(github_user.source_type, UserSourceType::GitHub);
}

#[test]
fn get_user_normalizes_legacy_github_fallback_last_name() {
    let mut repository = SqliteUserRepository::connect(":memory:")
        .expect("in-memory user repository should initialize");
    let mut user = seed_user(&mut repository);
    user.first_name = "CloudS3n".into();
    user.last_name = "GitHub".into();
    user.display_name = "CloudS3n".into();
    repository
        .update_user(&user)
        .expect("legacy GitHub fallback profile should be persisted");

    let link_id = repository
        .next_id()
        .expect("oauth identity id should be generated");
    repository
        .upsert_oauth_identity(&OAuth2IdentityLink {
            id: link_id,
            user_id: user.id.value(),
            user_code: user.user_code.clone(),
            provider_code: "github".into(),
            provider_subject: "github-legacy-user".into(),
            email: user.email.clone(),
            display_name: user.display_name.clone(),
            linked_at: 1_778_544_000_000,
            updated_at: 1_778_544_000_000,
        })
        .expect("oauth identity should be linked");

    let normalized_user = repository
        .get_user("USR-0001")
        .expect("seeded user should still exist");
    assert_eq!(normalized_user.source_type, UserSourceType::GitHub);
    assert_eq!(normalized_user.last_name, "CloudS3n");
}
