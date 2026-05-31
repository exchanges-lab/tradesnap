use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging via tracing-subscriber
    tracing_subscriber::fmt::init();

    info!("Starting TradeSnap application...");

    // Initialize configuration
    let config = Config::new()?;
    info!("Configuration loaded. RUST_LOG={}", config.rust_log);

    // Initialize persistent scraper singleton
    let scraper = TradingViewScraper::new(config)?;
    let state = Arc::new(AppState {
        scraper: std::sync::Mutex::new(scraper),
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
    info!("GET /chart request received for ticker={}, interval={}", params.ticker, params.interval);

    let ticker = params.ticker.clone();
    let interval = params.interval.clone();
    
    // Spawn browser session in a blocking task since headless_chrome has blocking API
    let res = tokio::task::spawn_blocking(move || {
        let mut scraper_lock = state.scraper.lock().map_err(|e| anyhow::anyhow!("Mutex poison error: {}", e))?;
        
        let run_result = if scraper_lock.config.use_save_shortcut {
            info!("Using save shortcut method to capture chart image data...");
            scraper_lock.get_chart_image_url(&ticker, &interval)
        } else {
            info!("Using traditional click method to capture chart screenshot link...");
            scraper_lock.get_screenshot_link(&ticker, &interval)
        };

        match run_result {
            Ok(url) => {
                let png_url = if scraper_lock.config.use_save_shortcut {
                    url.clone()
                } else {
                    TradingViewScraper::convert_link_to_image_url(&url).unwrap_or_else(|| url.clone())
                };
                Ok::<_, anyhow::Error>((url, png_url))
            }
            Err(e) => {
                error!("Scraping failed: {:?}. Attempting to recreate browser and retry...", e);
                // Recreate the scraper
                match TradingViewScraper::new(scraper_lock.config.clone()) {
                    Ok(new_scraper) => {
                        *scraper_lock = new_scraper;
                        // Retry
                        let retry_result = if scraper_lock.config.use_save_shortcut {
                            scraper_lock.get_chart_image_url(&ticker, &interval)?
                        } else {
                            scraper_lock.get_screenshot_link(&ticker, &interval)?
                        };
                        let png_url = if scraper_lock.config.use_save_shortcut {
                            retry_result.clone()
                        } else {
                            TradingViewScraper::convert_link_to_image_url(&retry_result).unwrap_or_else(|| retry_result.clone())
                        };
                        Ok((retry_result, png_url))
                    }
                    Err(recreate_err) => {
                        Err(anyhow::anyhow!("Failed to recreate scraper after failure: {}. Original error: {}", recreate_err, e))
                    }
                }
            }
        }
    }).await;


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
