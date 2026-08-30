use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use tradesnap::{Config, TradingViewScraper};

#[derive(Deserialize)]
struct ChartParams {
    ticker: String,
    interval: String,
}

#[derive(Serialize)]
struct ChartResponse {
    ticker: String,
    interval: String,
    image_url: String,
    png_url: String,
}

struct AppState {
    scraper: std::sync::Mutex<TradingViewScraper>,
    request_timeout: Duration,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env BEFORE tracing init so RUST_LOG is available
    let _ = dotenvy::dotenv();

    // Initialize structured logging with env filter (respects RUST_LOG, defaults to info)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting TradeSnap application...");

    // Initialize configuration
    let config = Config::new()?;
    info!("Configuration loaded successfully");

    // Initialize persistent scraper singleton
    let request_timeout = Duration::from_secs(config.request_timeout_seconds);
    let scraper = TradingViewScraper::new(config)?;
    let state = Arc::new(AppState {
        scraper: std::sync::Mutex::new(scraper),
        request_timeout,
    });

    // Build routes
    let app = Router::new()
        .route("/", get(health_check))
        .route("/chart", get(get_chart))
        .with_state(state);

    // Run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8003));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "{\"status\": \"ok\"}"
}

async fn get_chart(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ChartParams>,
) -> Result<Json<ChartResponse>, impl IntoResponse> {
    info!(
        "GET /chart request received for ticker={}, interval={}",
        params.ticker, params.interval
    );

    let ticker = params.ticker.clone();
    let interval = params.interval.clone();

    // Spawn browser session in a blocking task since headless_chrome has blocking API
    let request_timeout = state.request_timeout;
    let mut task = tokio::task::spawn_blocking(move || {
        let mut scraper_lock = state
            .scraper
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poison error: {}", e))?;

        // Pre-flight check: if the browser or tab is not alive, recreate it immediately!
        if !scraper_lock.is_alive() {
            info!("Scraper session is unresponsive. Recreating browser instance...");
            match TradingViewScraper::new(scraper_lock.config.clone()) {
                Ok(new_scraper) => {
                    *scraper_lock = new_scraper;
                }
                Err(err) => {
                    error!(
                        "Failed to recreate scraper during pre-flight check: {:?}",
                        err
                    );
                }
            }
        }

        let run_result = if scraper_lock.config.use_save_shortcut {
            info!("Using save shortcut method to capture chart image data...");
            scraper_lock.get_chart_image_url(&ticker, &interval)
        } else {
            info!("Using traditional click method to capture chart screenshot link...");
            scraper_lock.get_screenshot_link(&ticker, &interval)
        };

        let url = run_result?;
        let png_url = if scraper_lock.config.use_save_shortcut {
            url.clone()
        } else {
            TradingViewScraper::convert_link_to_image_url(&url).unwrap_or_else(|| url.clone())
        };
        Ok::<_, anyhow::Error>((url, png_url))
    });

    let res = match tokio::time::timeout(request_timeout, &mut task).await {
        Ok(result) => result,
        Err(_) => {
            error!(
                timeout_seconds = request_timeout.as_secs(),
                "Screenshot request timed out; terminating Chromium"
            );
            TradingViewScraper::terminate_profile_chromium();
            // Give the killed CDP session a brief chance to unwind and release
            // the scraper mutex. The HTTP deadline has already been reached.
            let _ = tokio::time::timeout(Duration::from_secs(2), &mut task).await;
            return Err((
                StatusCode::GATEWAY_TIMEOUT,
                format!(
                    "Screenshot request timed out after {} seconds",
                    request_timeout.as_secs()
                ),
            ));
        }
    };

    match res {
        Ok(Ok((image_url, png_url))) => {
            info!("Successfully captured chart!");
            Ok(Json(ChartResponse {
                ticker: params.ticker,
                interval: params.interval,
                image_url,
                png_url,
            }))
        }
        Ok(Err(err)) => {
            error!("Scraping failed: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Scraping failed: {}", err),
            ))
        }
        Err(join_err) => {
            error!("Thread join error: {:?}", join_err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server join error: {}", join_err),
            ))
        }
    }
}
