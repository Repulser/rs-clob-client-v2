#![cfg(feature = "clob")]

use polymarket_client_sdk_v2::clob::Config;

#[test]
fn clob_config_force_http2_defaults_off_and_can_be_enabled() {
    let default_cfg = Config::default();
    assert!(format!("{default_cfg:?}").contains("force_http2: false"));
    assert!(format!("{default_cfg:?}").contains("pool_idle_timeout: 300s"));
    assert!(format!("{default_cfg:?}").contains("http2_keep_alive_interval: 15s"));
    assert!(format!("{default_cfg:?}").contains("http2_keep_alive_timeout: 5s"));
    assert!(format!("{default_cfg:?}").contains("http2_keep_alive_while_idle: true"));

    let force_http2_cfg = Config::builder().force_http2(true).build();
    assert!(format!("{force_http2_cfg:?}").contains("force_http2: true"));
}
