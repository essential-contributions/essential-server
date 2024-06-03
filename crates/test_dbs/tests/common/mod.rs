macro_rules! create_test {
    ($func:ident) => {
        paste::paste! {
            #[tokio::test]
            async fn [<test_ $func>]() {
                #[cfg(feature = "rqlite")]
                $func(rqlite::TestRqlite::new().await.rqlite).await;
                $func(MemoryStorage::new()).await;
            }
        }
    };
}
pub(crate) use create_test;
