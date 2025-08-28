mod csv_writer;
mod excel_writer;
mod models;
mod scraper;

use anyhow::Result;
use std::env;
use tracing::{error, info};
use tracing_subscriber;

use crate::csv_writer::CsvExporter;
use crate::excel_writer::ExcelExporter;
use crate::scraper::{scrape_with_retry, VestbeeScraper};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Vestbee LP List Scraper");

    let args: Vec<String> = env::args().collect();
    let headless = !args.contains(&"--headed".to_string());
    
    if !headless {
        info!("Running in headed mode (browser visible)");
    }

    let scraper = VestbeeScraper::new(headless).await?;
    
    info!("Fetching fund URLs from list page");
    let fund_urls = scraper.get_fund_urls().await?;
    
    if fund_urls.is_empty() {
        error!("No fund URLs found. The page structure may have changed.");
        return Ok(());
    }
    
    info!("Found {} funds to scrape", fund_urls.len());

    let mut csv_writer = CsvExporter::new("data/vestbee_funds.csv")?;
    csv_writer.write_header()?;
    
    let mut all_funds = Vec::new();

    let mut successful_count = 0;
    let mut failed_count = 0;

    for (idx, url) in fund_urls.iter().enumerate() {
        info!("[{}/{}] Scraping: {}", idx + 1, fund_urls.len(), url);
        
        match scrape_with_retry(&scraper, url, 3).await {
            Ok(fund) => {
                if !fund.fund_name.is_empty() {
                    csv_writer.write_fund(&fund)?;
                    info!("Successfully scraped: {}", fund.fund_name);
                    all_funds.push(fund);
                    successful_count += 1;
                } else {
                    failed_count += 1;
                    error!("Scraped fund but name was empty for URL: {}", url);
                }
            }
            Err(e) => {
                failed_count += 1;
                error!("Failed to scrape {}: {}", url, e);
            }
        }
        
        if idx < fund_urls.len() - 1 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    csv_writer.finalize()?;
    
    // Write all funds to Excel
    let mut excel_writer = ExcelExporter::new()?;
    excel_writer.write_funds(&all_funds)?;
    excel_writer.save("data/vestbee_funds.xlsx")?;
    scraper.close().await?;

    info!(
        "Scraping complete! Successfully scraped {} funds, {} failed. Data saved to data/vestbee_funds.csv and data/vestbee_funds.xlsx",
        successful_count, failed_count
    );

    Ok(())
}
