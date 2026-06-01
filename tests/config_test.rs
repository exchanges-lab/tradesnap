use tradesnap::Config;

#[tokio::test]
async fn test_config_load_from_env() {
    // Set environment variables for testing
    unsafe {
        std::env::set_var("RUST_LOG", "trace");
        std::env::set_var("TRADINGVIEW_SESSION_ID", "dummy_session_id");
        std::env::set_var("TRADINGVIEW_SESSION_ID_SIGN", "dummy_session_id_sign");
    }

    // Initialize config
    let config = Config::new();
    assert!(config.is_ok());

    let config = config.unwrap();
    assert_eq!(config.rust_log, "trace");
    assert_eq!(config.session_id, "dummy_session_id");
    assert_eq!(config.session_id_sign, "dummy_session_id_sign");
}

#[test]
fn test_convert_link_to_image_url() {
    let input = "https://www.tradingview.com/x/abCdEfGh/";
    let output = tradesnap::TradingViewScraper::convert_link_to_image_url(input);
    assert_eq!(
        output,
        Some("https://s3.tradingview.com/snapshots/a/abCdEfGh.png".to_string())
    );

    let input_regional = "https://in.tradingview.com/x/abCdEfG";
    let output_regional = tradesnap::TradingViewScraper::convert_link_to_image_url(input_regional);
    assert_eq!(
        output_regional,
        Some("https://s3.tradingview.com/snapshots/a/abCdEfG.png".to_string())
    );
}

#[test]
fn test_map_interval() {
    use tradesnap::TradingViewScraper;
    assert_eq!(TradingViewScraper::map_interval("15"), "15");
    assert_eq!(TradingViewScraper::map_interval("15m"), "15");
    assert_eq!(TradingViewScraper::map_interval("1h"), "60");
    assert_eq!(TradingViewScraper::map_interval("4H"), "240");
    assert_eq!(TradingViewScraper::map_interval("1D"), "D");
    assert_eq!(TradingViewScraper::map_interval("d"), "D");
    assert_eq!(TradingViewScraper::map_interval("1W"), "W");
    assert_eq!(TradingViewScraper::map_interval("1M"), "M");
    assert_eq!(TradingViewScraper::map_interval("1m"), "1");
}
