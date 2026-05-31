use anyhow::{anyhow, Result};
use headless_chrome::{Browser, LaunchOptions, Tab};
use headless_chrome::protocol::cdp::Network::CookieParam;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;
use crate::Config;

/// A scraper for capturing TradingView chart screenshots using headless Chrome.
pub struct TradingViewScraper {
    _browser: Browser,
    tab: std::sync::Arc<Tab>,
    pub config: Config,
}

impl TradingViewScraper {
    /// Initialises a new `TradingViewScraper` with the provided configuration.
    pub fn new(config: Config) -> Result<Self> {
        info!("Initializing TradingViewScraper...");
        
        // Create custom Chrome user data directory to write preferences
        let profile_dir = std::env::current_dir()
            .map_err(|e| anyhow!("Failed to get current directory: {}", e))?
            .join("target/chrome_profile");
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
        info!("Written custom Chrome Preferences to: {:?}", preferences_file);

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
            OsStr::new("--disable-renderer-backgrounding"),
            OsStr::new("--disable-background-timer-throttling"),
            OsStr::new("--disable-backgrounding-occluded-windows"),
            OsStr::new("--disable-hang-monitor"),
            OsStr::new(&window_size_arg),
        ];
        builder.args(browser_args);

        // Specify snap Chromium path if it exists
        let snap_chromium = PathBuf::from("/snap/bin/chromium");
        if snap_chromium.exists() {
            info!("Using snap Chromium at: {:?}", snap_chromium);
            builder.path(Some(snap_chromium));
        }

        let options = builder.build().map_err(|e| anyhow!("Failed to build launch options: {}", e))?;
        let browser = Browser::new(options).map_err(|e| anyhow!("Failed to launch browser: {}", e))?;
        
        let tab = browser.new_tab().map_err(|e| anyhow!("Failed to get tab: {}", e))?;

        let scraper = Self {
            _browser: browser,
            tab,
            config,
        };

        if !scraper.config.session_id.is_empty() {
            scraper.set_auth_cookies()?;
        }

        Ok(scraper)
    }

    /// Sets session authentication cookies.
    pub fn set_auth_cookies(&self) -> Result<()> {
        info!("Setting session auth cookies...");
        let cookie1 = CookieParam {
            name: "sessionid".to_string(),
            value: self.config.session_id.clone(),
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
            value: self.config.session_id_sign.clone(),
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

        self.tab.set_cookies(vec![cookie1, cookie2])
            .map_err(|e| anyhow!("Failed to set cookies: {}", e))?;
        
        Ok(())
    }

    /// Maps human-readable intervals (e.g. "1h", "4h", "1D") to TradingView expected formats ("60", "240", "D").
    pub fn map_interval(interval: &str) -> String {
        let trimmed = interval.trim();
        match trimmed {
            "1h" | "1H" => "60".to_string(),
            "2h" | "2H" => "120".to_string(),
            "4h" | "4H" => "240".to_string(),
            "1d" | "1D" | "d" | "D" => "D".to_string(),
            "1w" | "1W" | "w" | "W" => "W".to_string(),
            "1M" => "M".to_string(),
            "1m" => "1".to_string(),
            other => {
                if (other.ends_with('m') || other.ends_with('M')) && other[..other.len()-1].chars().all(|c| c.is_ascii_digit()) {
                    other[..other.len()-1].to_string()
                } else {
                    trimmed.to_string()
                }
            }
        }
    }

    /// Navigates to a chart page and waits for its elements.
    pub fn navigate_to_chart(&self, ticker: &str, interval: &str, use_layout: bool) -> Result<()> {
        let mapped_interval = Self::map_interval(interval);
        let chart_url = if use_layout {
            format!(
                "https://in.tradingview.com/chart/{}/?symbol={}&interval={}&theme=light",
                self.config.chart_page_id, ticker, mapped_interval
            )
        } else {
            format!(
                "https://in.tradingview.com/chart/?symbol={}&interval={}&theme=light",
                ticker, mapped_interval
            )
        };

        info!("Navigating to chart: {}", chart_url);
        self.tab.navigate_to(&chart_url).map_err(|e| anyhow!("Failed to navigate: {}", e))?;

        info!("Waiting for chart elements to render...");
        let element_selector = "#header-toolbar-chart-styles, .tv-header, [data-name='legend-source-item'], .chart-container, .tv-chart-container";
        self.tab.wait_for_element(element_selector)
            .map_err(|e| anyhow!("Timed out waiting for chart infrastructure: {}", e))?;
        
        Ok(())
    }


    /// Focuses the page by clicking the main canvas element.
    pub fn focus_page(&self) -> Result<()> {
        info!("Finding main canvas element...");
        let canvas = self.tab.wait_for_element("canvas")
            .map_err(|e| anyhow!("Failed to find canvas: {}", e))?;
        
        info!("Clicking canvas to focus...");
        canvas.click().map_err(|e| anyhow!("Failed to click canvas: {}", e))?;
        std::thread::sleep(Duration::from_millis(500));
        
        Ok(())
    }

    /// Triggers screenshot capture by clicking the toolbar screenshot button,
    /// selecting "Copy link" option, and returning the raw screenshot share link.
    pub fn get_screenshot_link(&self, ticker: &str, interval: &str) -> Result<String> {
        let use_layout = !self.config.session_id.is_empty();
        self.navigate_to_chart(ticker, interval, use_layout)?;
        self.focus_page()?;

        info!("Finding screenshot button...");
        let screenshot_btn = self.tab.wait_for_element("#header-toolbar-screenshot")
            .map_err(|e| anyhow!("Failed to find screenshot button: {}", e))?;

        info!("Clicking screenshot button...");
        screenshot_btn.click().map_err(|e| anyhow!("Failed to click screenshot button: {}", e))?;

        // Wait for dropdown menu to render
        std::thread::sleep(Duration::from_millis(800));

        info!("Finding 'Copy link' element using XPath...");
        let copy_link_el = self.tab.wait_for_xpath("//span[text()='Copy link']")
            .map_err(|e| anyhow!("Failed to find 'Copy link' element: {}", e))?;

        info!("Clicking 'Copy link' element natively...");
        copy_link_el.click().map_err(|e| anyhow!("Failed to click 'Copy link': {}", e))?;

        // Poll the clipboard for the copied URL
        info!("Polling clipboard for the screenshot link...");
        let mut clipboard_url = None;
        for _ in 1..=15 {
            let read_attempt = self.tab.evaluate("navigator.clipboard.readText()", true)
                .ok()
                .and_then(|obj| obj.value)
                .and_then(|val| val.as_str().map(|s| s.trim().to_string()));

            if let Some(trimmed) = read_attempt.filter(|t| t.contains("tradingview.com/x/")) {
                info!("Found link in clipboard: {}", trimmed);
                clipboard_url = Some(trimmed);
                break;
            }
            std::thread::sleep(Duration::from_millis(1000));
        }

        match clipboard_url {
            Some(url) => Ok(url),
            None => Err(anyhow!("Failed to retrieve screenshot link from clipboard after retries")),
        }
    }

    /// Triggers image capture using keyboard shortcut (Ctrl+Shift+S) and
    /// returns the clipboard image as a base64 encoded data URL.
    pub fn get_chart_image_url(&self, ticker: &str, interval: &str) -> Result<String> {
        let use_layout = !self.config.session_id.is_empty();
        self.navigate_to_chart(ticker, interval, use_layout)?;
        self.focus_page()?;

        info!("Sending Shift+Ctrl+S keypress to capture image...");
        let trigger_js = r#"
            const event = new KeyboardEvent('keydown', {
                key: 's',
                code: 'KeyS',
                keyCode: 83,
                ctrlKey: true,
                shiftKey: true,
                bubbles: true,
                cancelable: true
            });
            document.dispatchEvent(event);
        "#;
        self.tab.evaluate(trigger_js, false)
            .map_err(|e| anyhow!("Failed to trigger screenshot shortcut: {}", e))?;

        // Wait for clipboard to populate
        std::thread::sleep(Duration::from_millis(800));

        info!("Polling clipboard for image data...");
        let mut image_data_url = None;
        for _ in 1..=10 {
            if let Ok(Some(data_url)) = self.read_image_from_clipboard() {
                image_data_url = Some(data_url);
                break;
            }
            std::thread::sleep(Duration::from_millis(800));
        }

        match image_data_url {
            Some(url) => Ok(url),
            None => Err(anyhow!("Failed to retrieve chart image from clipboard after retries")),
        }
    }

    /// Evaluates async JavaScript to read binary image data from clipboard.
    fn read_image_from_clipboard(&self) -> Result<Option<String>> {
        let js = r#"
            (async () => {
                try {
                    const items = await navigator.clipboard.read();
                    for (const item of items) {
                        for (const type of item.types) {
                            if (type.startsWith('image/')) {
                                const blob = await item.getType(type);
                                return new Promise((resolve, reject) => {
                                    const reader = new FileReader();
                                    reader.onload = () => resolve(reader.result);
                                    reader.onerror = (e) => reject(e);
                                    reader.readAsDataURL(blob);
                                });
                            }
                        }
                    }
                } catch (e) {
                    console.error("Clipboard image read error:", e);
                }
                return null;
            })()
        "#;
        
        let obj = self.tab.evaluate(js, true)
            .map_err(|e| anyhow!("Failed to evaluate clipboard read: {}", e))?;
        if let Some(s) = obj.value.as_ref().and_then(|v| v.as_str()).filter(|s| s.starts_with("data:image/")) {
            return Ok(Some(s.to_string()));
        }
        Ok(None)
    }

    /// Converts a raw TradingView share link (e.g. `/x/`) to a direct S3 snapshot image link.
    pub fn convert_link_to_image_url(input_string: &str) -> Option<String> {
        let re = regex::Regex::new(r"https://(?:www\.|in\.)?tradingview\.com/x/([a-zA-Z0-9]+)/?").ok()?;
        if let Some(caps) = re.captures(input_string) {
            let id = caps.get(1)?.as_str();
            let first_char = id.chars().next()?.to_lowercase().to_string();
            Some(format!("https://s3.tradingview.com/snapshots/{}/{}.png", first_char, id))
        } else {
            None
        }
    }

    /// Verifies if the browser and tab connection are still active and responding.
    pub fn is_alive(&self) -> bool {
        self.tab.evaluate("1", false).is_ok()
    }
}
