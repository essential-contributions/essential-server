use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use super::*;

#[tokio::test]
async fn test_err() {
    let notify = Notify::new();
    let rx = notify.subscribe_contracts();
    let (result, state) =
        next_data::<_, _, ()>(rx.clone(), StreamState::default(), 100, |_| async {
            Err(anyhow::anyhow!("error"))
        })
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert!(result[0].is_err());

    let result = next_data::<_, _, ()>(rx.clone(), state.clone(), 100, |_| async {
        Err(anyhow::anyhow!("error"))
    })
    .await;
    assert!(result.is_none());

    let result =
        next_data::<_, _, ()>(rx.clone(), state.clone(), 100, |_| async { Ok(vec![()]) }).await;
    assert!(result.is_none());
}

#[tokio::test]
async fn no_data() {
    let notify = Notify::new();
    let rx = notify.subscribe_contracts();
    let num = Arc::new(AtomicUsize::new(0));
    notify.notify_new_contracts();
    let (result, state) =
        next_data::<_, _, ()>(rx.clone(), StreamState::default(), 100, |_| async {
            if num.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(vec![])
            } else {
                Ok(vec![()])
            }
        })
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert!(result[0].is_ok());
    assert_eq!(num.load(Ordering::SeqCst), 2);
    assert_eq!(
        state,
        StreamState {
            state: State::Pos(Pos { page: 0, index: 1 }),
            start: Default::default()
        }
    );
}

#[tokio::test]
async fn page_calc() {
    let notify = Notify::new();
    let rx = notify.subscribe_contracts();
    let (result, state) = next_data::<_, _, ()>(rx.clone(), StreamState::default(), 3, |_| async {
        Ok(vec![()])
    })
    .await
    .unwrap();

    assert_eq!(result.len(), 1);
    assert!(result[0].is_ok());
    assert_eq!(
        state,
        StreamState {
            state: State::Pos(Pos { page: 0, index: 1 }),
            start: Default::default()
        }
    );

    let (result, state) =
        next_data::<_, _, ()>(rx.clone(), state, 3, |_| async { Ok(vec![(); 3]) })
            .await
            .unwrap();

    assert_eq!(result.len(), 2);
    assert!(result.iter().all(|r| r.is_ok()));
    assert_eq!(
        state,
        StreamState {
            state: State::Pos(Pos { page: 1, index: 0 }),
            start: Default::default()
        }
    );

    let (result, state) =
        next_data::<_, _, ()>(rx.clone(), state, 3, |_| async { Ok(vec![(); 3]) })
            .await
            .unwrap();

    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|r| r.is_ok()));
    assert_eq!(
        state,
        StreamState {
            state: State::Pos(Pos { page: 2, index: 0 }),
            start: Default::default()
        }
    );

    let (_, state) = next_data::<_, _, ()>(rx.clone(), state, 3, |_| async { Ok(vec![(); 2]) })
        .await
        .unwrap();

    let num = Arc::new(AtomicUsize::new(0));
    notify.notify_new_contracts();
    let (result, state) = next_data::<_, _, ()>(rx.clone(), state, 3, |get| {
        let num = num.clone();
        async move {
            assert_eq!(get.page, 2);
            if num.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(vec![(); 2])
            } else {
                Ok(vec![(); 3])
            }
        }
    })
    .await
    .unwrap();

    assert_eq!(result.len(), 1);
    assert!(result.iter().all(|r| r.is_ok()));
    assert_eq!(
        state,
        StreamState {
            state: State::Pos(Pos { page: 3, index: 0 }),
            start: Default::default()
        }
    );

    let (_, state) = next_data::<_, _, ()>(rx.clone(), state, 3, |_| async { Ok(vec![(); 1]) })
        .await
        .unwrap();

    let num = Arc::new(AtomicUsize::new(0));
    notify.notify_new_contracts();
    let (result, state) = next_data::<_, _, ()>(rx.clone(), state, 3, |get| {
        let num = num.clone();
        async move {
            assert_eq!(get.page, 3);
            if num.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(vec![(); 1])
            } else {
                Ok(vec![(); 2])
            }
        }
    })
    .await
    .unwrap();

    assert_eq!(result.len(), 1);
    assert!(result.iter().all(|r| r.is_ok()));
    assert_eq!(
        state,
        StreamState {
            state: State::Pos(Pos { page: 3, index: 2 }),
            start: Default::default()
        }
    );
}

#[tokio::test]
async fn ordering() {
    let notify = Notify::new();
    let rx = notify.subscribe_contracts();

    let (result, state) = next_data::<_, _, _>(rx.clone(), StreamState::default(), 3, |_| async {
        Ok(vec![1])
    })
    .await
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(*result[0].as_ref().unwrap(), 1);

    let (result, _) = next_data::<_, _, _>(rx.clone(), state, 3, |_| async { Ok(vec![1, 2, 3]) })
        .await
        .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(*result[0].as_ref().unwrap(), 2);
    assert_eq!(*result[1].as_ref().unwrap(), 3);
}
