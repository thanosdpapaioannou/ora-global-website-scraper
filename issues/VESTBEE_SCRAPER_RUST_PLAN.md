# VESTBEE LP LIST SCRAPER - RUST IMPLEMENTATION

## Objective
Build a Rust-based web scraper to extract fund information from https://www.vestbee.com/lp-list

## Data to Extract
- Fund Name
- Fund URL
- Investment Geographies
- Fund Description
- Fund Portfolio

## Technology Stack
- **Language**: Rust
- **Browser Automation**: chromiumoxide (Chrome DevTools Protocol)
- **CSV Writing**: csv crate
- **Async Runtime**: tokio
- **HTML Parsing**: scraper crate (as fallback)

## Implementation Steps

### Step 1: Project Setup
- Initialize Cargo project
- Configure dependencies
- Create project structure

### Step 2: Core Components
1. **models.rs** - Data structures for Fund information
2. **scraper.rs** - Browser automation and data extraction
3. **csv_writer.rs** - CSV export functionality
4. **main.rs** - Application orchestration

### Step 3: Scraping Logic
1. Navigate to LP list page
2. Extract all "Details" button URLs
3. Visit each fund page
4. Extract required data fields
5. Handle errors and retries
6. Export to CSV

### Step 4: Error Handling
- Implement retry logic for failed requests
- Add rate limiting to avoid blocking
- Graceful error recovery

## Project Structure
```
vestbee-scraper/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── models.rs
│   ├── scraper.rs
│   └── csv_writer.rs
├── data/
│   └── vestbee_funds.csv
└── issues/
    └── VESTBEE_SCRAPER_RUST_PLAN.md
```

## Expected Output
CSV file with columns:
- fund_name
- fund_url
- investment_geographies
- fund_description
- fund_portfolio