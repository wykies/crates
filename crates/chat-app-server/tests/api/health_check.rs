use crate::helpers::{no_cb, spawn_app};

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let actual = app.core_client.health_check(no_cb).await.unwrap();

    // Assert
    // Using unwrap so error shows instead of asserting `is_ok``
    actual.unwrap();
}
