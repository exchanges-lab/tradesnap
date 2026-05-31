use anyhow::{anyhow, Result};
use headless_chrome::{Browser, LaunchOptions};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::Duration;
use tradesnap::Config;

// Import the auto-generated CDP types (directly inside domain modules)
use headless_chrome::protocol::cdp::Network::CookieParam;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load config
    let config = Config::new()?;
    println!("Loaded config: symbol=BYBIT:BTCUSDT.P, chart_page_id={}", config.chart_page_id);

    // Create a custom Chrome user data directory to write preferences
    let profile_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).join("target/chrome_profile");
    let preferences_dir = profile_dir.join("Default");
    std::fs::create_dir_all(&preferences_dir)?;

    let preferences_file = preferences_dir.join("Preferences");
    let prefs_json = serde_json::json!({
        "profile": {
            "content_settings": {
                "exceptions": {
                    "clipboard": {
                        "https://in.tradingview.com,*": { "setting": 1 },
                        "https://www.tradingview.com,*": { "setting": 1 },
                        "[*.]tradingview.com,*": { "setting": 1 }
                    }
                }
            }
        }
    });
    std::fs::write(&preferences_file, serde_json::to_string(&prefs_json)?)?;
    println!("Written custom Chrome Preferences to: {:?}", preferences_file);

    // Setup browser launch options
    let mut builder = LaunchOptions::default_builder();
    builder.headless(config.headless);
    builder.user_data_dir(Some(profile_dir));

    let window_size_arg = format!("--window-size={},{}", config.window_width, config.window_height);
    let browser_args = vec![
        OsStr::new("--no-sandbox"),
        OsStr::new("--disable-dev-shm-usage"),
        OsStr::new("--disable-gpu"),
        OsStr::new("--enable-clipboard-read-write"),
        OsStr::new("--disable-web-security"),
        OsStr::new("--allow-running-insecure-content"),
        OsStr::new(&window_size_arg),
    ];
    builder.args(browser_args);

    // Specify the path to snap chromium if it exists
    let snap_chromium = PathBuf::from("/snap/bin/chromium");
    if snap_chromium.exists() {
        println!("Using snap Chromium at: {:?}", snap_chromium);
        builder.path(Some(snap_chromium));
    }

    let options = builder.build().map_err(|e| anyhow!("Failed to build launch options: {}", e))?;
    println!("Launching browser...");
    let browser = Browser::new(options).map_err(|e| anyhow!("Failed to launch browser: {}", e))?;

    println!("Opening new tab...");
    let tab = browser.new_tab().map_err(|e| anyhow!("Failed to get tab: {}", e))?;

    let ticker = "BYBIT:BTCUSDT.P";
    let interval = "15";
    let chart_url = format!(
        "https://in.tradingview.com/chart/?symbol={}&interval={}&theme=light",
        ticker, interval
    );

    // Navigate to chart domain first (initial visit)
    println!("Navigating to chart domain: {} ...", chart_url);
    tab.navigate_to(&chart_url).map_err(|e| anyhow!("Failed to navigate: {}", e))?;

    // Set auth cookies using tab.set_cookies
    println!("Setting auth cookies...");
    
    let cookie1 = CookieParam {
        name: "sessionid".to_string(),
        value: config.session_id.clone(),
        url: Some("https://in.tradingview.com".to_string()),
        domain: Some(".tradingview.com".to_string()),
        path: Some("/".to_string()),
        secure: Some(true),
        http_only: Some(true),
        same_site: None,
        expires: None,
        priority: None,
        same_party: None,
        source_scheme: None,
        source_port: None,
        partition_key: None,
    };

    let cookie2 = CookieParam {
        name: "sessionid_sign".to_string(),
        value: config.session_id_sign.clone(),
        url: Some("https://in.tradingview.com".to_string()),
        domain: Some(".tradingview.com".to_string()),
        path: Some("/".to_string()),
        secure: Some(true),
        http_only: Some(true),
        same_site: None,
        expires: None,
        priority: None,
        same_party: None,
        source_scheme: None,
        source_port: None,
        partition_key: None,
    };

    tab.set_cookies(vec![cookie1, cookie2])
        .map_err(|e| anyhow!("Failed to set cookies: {}", e))?;

    // Refresh page to apply cookies
    println!("Refreshing page to apply cookies...");
    tab.reload(false, None).map_err(|e| anyhow!("Failed to reload page: {}", e))?;

    // Print cookies on the page for debugging
    if let Ok(cookie_val) = tab.evaluate("document.cookie", true) {
        println!("Cookies on page (JS): {:?}", cookie_val.value);
    }
    match tab.get_cookies() {
        Ok(cookies) => {
            println!("Cookies in browser tab context:");
            for c in cookies {
                println!("  Cookie: name={}, domain={}, path={}, secure={}, http_only={}", c.name, c.domain, c.path, c.secure, c.http_only);
            }
        }
        Err(e) => {
            println!("Failed to get cookies: {:?}", e);
        }
    }

    // Wait for the chart elements to render
    println!("Waiting for chart elements...");
    let element_selector = "#header-toolbar-chart-styles, .tv-header, [data-name='legend-source-item'], .chart-container, .tv-chart-container";
    let _ = tab.wait_for_element(element_selector)
        .map_err(|e| anyhow!("Timed out waiting for chart infrastructure: {}", e))?;
    println!("Chart infrastructure is ready.");

    // Capture screenshot at ready state
    if let Ok(screenshot_data) = tab.capture_screenshot(
        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
        None,
        None,
        true,
    ) {
        let path = "screenshot.png";
        let _ = std::fs::write(path, screenshot_data);
        println!("Saved ready state screenshot to {}", path);
    }

    // Find and click the canvas to focus page
    println!("Finding main canvas element...");
    let canvas_el = tab.wait_for_element("canvas")
        .map_err(|e| anyhow!("Failed to find canvas: {}", e))?;

    println!("Clicking canvas to focus...");
    canvas_el.click().map_err(|e| anyhow!("Failed to click canvas: {}", e))?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Click screenshot button using native CDP click
    println!("Finding screenshot button...");
    let screenshot_btn = tab.wait_for_element("#header-toolbar-screenshot")
        .map_err(|e| anyhow!("Failed to find screenshot button: {}", e))?;

    println!("Clicking screenshot button...");
    screenshot_btn.click().map_err(|e| anyhow!("Failed to click screenshot button: {}", e))?;

    // Wait for menu to render
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Find the "Copy link" option using XPath
    println!("Finding 'Copy link' element using XPath...");
    let copy_link_el = tab.wait_for_xpath("//span[text()='Copy link']")
        .map_err(|e| anyhow!("Failed to find 'Copy link' element: {}", e))?;

    println!("Clicking 'Copy link' element natively...");
    copy_link_el.click().map_err(|e| anyhow!("Failed to click 'Copy link' element: {}", e))?;

    // Wait and poll clipboard
    let mut clipboard_url = None;
    for i in 1..=15 {
        println!("Checking clipboard (attempt {}/15)...", i);
        match tab.evaluate("navigator.clipboard.readText()", true) {
            Ok(remote_obj) => {
                println!("Attempt {} raw value: {:?}", i, remote_obj.value);
                let read_val = remote_obj.value
                    .and_then(|val| val.as_str().map(|s| s.trim().to_string()))
                    .filter(|trimmed| trimmed.contains("tradingview.com"));
                if let Some(trimmed) = read_val {
                    println!("Found link in clipboard: {}", trimmed);
                    clipboard_url = Some(trimmed);
                    break;
                }
            }
            Err(e) => {
                println!("Warning: failed to read clipboard via JS: {:?}", e);
            }
        }
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }

    if let Some(url) = clipboard_url {
        println!("SUCCESS! Retrieved clipboard URL: {}", url);
    } else {
        println!("FAILURE! Did not retrieve clipboard URL.");
        // Take a debug screenshot
        if let Ok(screenshot_data) = tab.capture_screenshot(
            headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
            None,
            None,
            true,
        ) {
            let path = "screenshot_after.png";
            if let Err(e) = std::fs::write(path, screenshot_data) {
                println!("Warning: failed to write screenshot_after.png: {:?}", e);
            } else {
                println!("Saved failure screenshot to {}", path);
            }
        }
    }

    Ok(())
}
