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
    config: Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging via tracing-subscriber
    tracing_subscriber::fmt::init();

    info!("Starting TradeSnap application...");

    // Initialize configuration
    let config = Config::new()?;
    info!("Configuration loaded. RUST_LOG={}", config.rust_log);

    let state = Arc::new(AppState { config });

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

    let config = state.config.clone();
    let ticker = params.ticker.clone();
    let interval = params.interval.clone();
    
    // Spawn browser session in a blocking task since headless_chrome has blocking API
    let res = tokio::task::spawn_blocking(move || {
        let scraper = TradingViewScraper::new(config)?;
        
        let (image_url, png_url) = if scraper.config.use_save_shortcut {
            info!("Using save shortcut method to capture chart image data...");
            let image_data = scraper.get_chart_image_url(&ticker, &interval)?;
            (image_data.clone(), image_data)
        } else {
            info!("Using traditional click method to capture chart screenshot link...");
            let raw_link = scraper.get_screenshot_link(&ticker, &interval)?;
            let png_link = TradingViewScraper::convert_link_to_image_url(&raw_link)
                .unwrap_or_else(|| raw_link.clone());
            (raw_link, png_link)
        };
        
        Ok::<_, anyhow::Error>((image_url, png_url))
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
