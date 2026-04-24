use quantedge_core::Ohlcv;

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

#[test]
fn ohlcv_is_send_and_sync() {
    assert_send::<Ohlcv>();
    assert_sync::<Ohlcv>();
}
