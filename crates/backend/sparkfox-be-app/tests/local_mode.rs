use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn test_local_mode_skips_auth() {
    let db = sparkfox_be_db::init_database_memory().await.unwrap();
    let config = sparkfox_be_app::AppConfig {
        auth_policy: sparkfox_be_app::AuthPolicy::NoAuth,
        ..Default::default()
    };
    let services = sparkfox_be_app::AppServices::from_config(db, &config).await.unwrap();

    let router = sparkfox_be_app::create_router(&services).await;

    // Health check should work
    let response = router
        .clone()
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // An authenticated endpoint should work WITHOUT a token in local mode
    let response = router
        .oneshot(Request::builder().uri("/api/settings").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_ne!(response.status(), StatusCode::FORBIDDEN);

    services.database.close().await;
}

#[tokio::test]
async fn test_non_local_mode_requires_auth() {
    let db = sparkfox_be_db::init_database_memory().await.unwrap();
    let services = sparkfox_be_app::AppServices::from_config(db, &sparkfox_be_app::AppConfig::default())
        .await
        .unwrap();

    let router = sparkfox_be_app::create_router(&services).await;

    let response = router
        .oneshot(Request::builder().uri("/api/settings").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    services.database.close().await;
}
