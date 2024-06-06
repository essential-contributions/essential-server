pub use paste;

#[cfg(feature = "rqlite")]
pub use rqlite::*;
#[cfg(feature = "rqlite")]
mod rqlite;

#[macro_export]
macro_rules! create_test {
    ($func:ident) => {
        $crate::paste::paste! {
            #[tokio::test]
            async fn [<test_ $func>]() {
                #[cfg(feature = "rqlite")]
                $func($crate::TestRqlite::new().await.rqlite).await;
                $func(essential_memory_storage::MemoryStorage::new()).await;
            }
        }
    };
}
