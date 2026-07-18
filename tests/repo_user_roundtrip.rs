use chrono::Utc;
use smart_hawk::adapters::crypto::AesGcmSecretBox;
use smart_hawk::adapters::db::{connect, SqliteUserRepository};
use smart_hawk::domain::{ApiKey, Locale, ModelId, ProviderKind, TelegramUserId, User};
use smart_hawk::ports::{SecretBox, UserRepository};

#[tokio::test]
async fn user_insert_update_delete() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let url = format!("sqlite:{}?mode=rwc", path.display());
    let pool = connect(&url).await.unwrap();
    let repo = SqliteUserRepository::new(pool);
    let secrets = AesGcmSecretBox::from_master_key_str(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    )
    .unwrap();

    let now = Utc::now();
    let mut user = User::new(TelegramUserId::new(42), "Tester".into(), Locale::En, now).unwrap();

    let key = ApiKey::new("sk-test-key").unwrap();
    let blob = secrets.encrypt(&key).unwrap();
    user.set_ai_partial(
        ProviderKind::OpenAi,
        None,
        blob,
        ModelId::new("gpt-test").unwrap(),
        now,
    )
    .unwrap();
    user.mark_ai_verified(now).unwrap();

    repo.insert(&user).await.unwrap();
    let loaded = repo
        .get_by_telegram_id(TelegramUserId::new(42))
        .await
        .unwrap()
        .expect("user");
    assert_eq!(loaded.display_name, "Tester");
    assert!(loaded.ai_config.as_ref().unwrap().is_verified());

    let plain = secrets
        .decrypt(&loaded.ai_config.as_ref().unwrap().api_key)
        .unwrap();
    assert_eq!(plain.expose(), "sk-test-key");

    repo.delete_by_id(user.id).await.unwrap();
    assert!(repo
        .get_by_telegram_id(TelegramUserId::new(42))
        .await
        .unwrap()
        .is_none());
}
