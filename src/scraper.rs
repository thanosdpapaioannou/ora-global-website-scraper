use anyhow::{Context, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::models::Fund;

pub struct VestbeeScraper {
    browser: Browser,
    base_url: String,
}

impl VestbeeScraper {
    pub async fn new(headless: bool) -> Result<Self> {
        info!("Initializing browser");
        
        let mut config = BrowserConfig::builder();
        if !headless {
            config = config.with_head();
        }
        config = config.window_size(1920, 1080);
        config = config.viewport(None);
        
        let browser_config = config.build()
            .map_err(|e| anyhow::anyhow!("Failed to build browser config: {}", e))?;
        
        let (browser, mut handler) = Browser::launch(browser_config)
            .await
            .context("Failed to launch browser")?;

        tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if let Err(e) = h {
                    error!("Browser handler error: {:?}", e);
                }
            }
        });

        Ok(Self {
            browser,
            base_url: "https://www.vestbee.com/lp-list".to_string(),
        })
    }

    pub async fn get_fund_urls(&self) -> Result<Vec<String>> {
        info!("Navigating to LP list page");
        let page = self.browser.new_page(&self.base_url).await?;
        
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        let mut all_fund_urls = Vec::new();
        let mut page_number = 1;
        let mut has_next_page = true;
        
        while has_next_page {
            info!("Scraping page {}", page_number);
            
            // Get fund URLs from current page
            let fund_urls = page
                .evaluate(
                    r#"
                    Array.from(document.querySelectorAll('a, button'))
                        .filter(el => el.innerText && el.innerText.includes('Details'))
                        .map(el => {
                            if (el.tagName === 'A' && el.href) {
                                return el.href;
                            } else if (el.onclick) {
                                return el.getAttribute('data-href') || el.getAttribute('href') || '';
                            }
                            const parent = el.closest('a');
                            if (parent && parent.href) {
                                return parent.href;
                            }
                            const card = el.closest('[data-href], [href]');
                            if (card) {
                                return card.getAttribute('data-href') || card.getAttribute('href') || '';
                            }
                            return '';
                        })
                        .filter(url => url && url.length > 0)
                        .map(url => {
                            if (url.startsWith('http')) return url;
                            if (url.startsWith('/')) return window.location.origin + url;
                            return window.location.origin + '/' + url;
                        })
                        .filter(url => !url.includes('undefined'));
                    "#,
                )
                .await?
                .into_value::<Vec<String>>()?;
            
            info!("Found {} funds on page {}", fund_urls.len(), page_number);
            
            // Add unique URLs to our collection
            for url in fund_urls {
                if !all_fund_urls.contains(&url) {
                    all_fund_urls.push(url);
                }
            }
            
            // Check if there's a next page and click it
            has_next_page = page
                .evaluate(
                    r#"
                    (() => {
                        // First try to find and click "Next" button
                        const nextButtons = Array.from(document.querySelectorAll('a, button'))
                            .filter(el => {
                                const text = (el.textContent || '').toLowerCase();
                                const ariaLabel = (el.getAttribute('aria-label') || '').toLowerCase();
                                return text === 'next' || 
                                       text.includes('next') || 
                                       ariaLabel.includes('next') ||
                                       text === '→' ||
                                       text === '>';
                            });
                        
                        for (const btn of nextButtons) {
                            // Check if next button is not disabled
                            if (!btn.disabled && !btn.classList.contains('disabled')) {
                                btn.click();
                                return true;
                            }
                        }
                        
                        // Alternative: Look for numbered pagination
                        const currentPage = document.querySelector('.pagination .active, [aria-current="page"]');
                        if (currentPage) {
                            const currentPageNum = parseInt(currentPage.textContent);
                            // Find link with next page number
                            const allLinks = Array.from(document.querySelectorAll('a'));
                            const nextPageLink = allLinks.find(link => link.textContent.trim() === String(currentPageNum + 1));
                            if (nextPageLink) {
                                nextPageLink.click();
                                return true;
                            }
                        }
                        
                        // Alternative: Look for page number links
                        const pageLinks = Array.from(document.querySelectorAll('a'))
                            .filter(el => {
                                const text = el.textContent || '';
                                return /^\d+$/.test(text.trim());
                            })
                            .sort((a, b) => parseInt(a.textContent) - parseInt(b.textContent));
                        
                        // Find current page and click next
                        for (let i = 0; i < pageLinks.length - 1; i++) {
                            if (pageLinks[i].classList.contains('active') || 
                                pageLinks[i].getAttribute('aria-current') === 'page') {
                                pageLinks[i + 1].click();
                                return true;
                            }
                        }
                        
                        return false;
                    })()
                    "#,
                )
                .await?
                .into_value::<bool>()?;
            
            if has_next_page {
                info!("Navigating to page {}", page_number + 1);
                tokio::time::sleep(Duration::from_secs(3)).await;
                page_number += 1;
                
                // Safety check - don't scrape more than 100 pages
                if page_number > 100 {
                    warn!("Reached maximum page limit of 100, stopping pagination");
                    break;
                }
            } else {
                info!("No more pages found, finished pagination");
            }
        }
        
        let fund_urls = all_fund_urls;

        if fund_urls.is_empty() {
            warn!("No fund URLs found, trying alternative selectors");
            
            let alternative_urls = page
                .evaluate(
                    r#"
                    Array.from(document.querySelectorAll('[class*="card"], [class*="item"], [class*="fund"]'))
                        .map(el => {
                            const link = el.querySelector('a[href]');
                            if (link) return link.href;
                            const dataHref = el.getAttribute('data-href');
                            if (dataHref) {
                                if (dataHref.startsWith('http')) return dataHref;
                                if (dataHref.startsWith('/')) return window.location.origin + dataHref;
                                return window.location.origin + '/' + dataHref;
                            }
                            return '';
                        })
                        .filter(url => url && url.length > 0);
                    "#,
                )
                .await?
                .into_value::<Vec<String>>()?;
            
            if !alternative_urls.is_empty() {
                info!("Found {} URLs using alternative selectors", alternative_urls.len());
                return Ok(alternative_urls);
            }
        }

        info!("Found {} fund URLs", fund_urls.len());
        Ok(fund_urls)
    }

    pub async fn scrape_fund_details(&self, url: &str) -> Result<Fund> {
        info!("Scraping fund details from: {}", url);
        let page = self.browser.new_page(url).await?;
        
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        let mut fund = Fund::new();
        fund.fund_url = url.to_string();

        let fund_name = page
            .evaluate(
                r#"
                (() => {
                    const selectors = ['h1', '.fund-name', '.company-name', '.title', '[class*="name"]'];
                    for (const selector of selectors) {
                        const el = document.querySelector(selector);
                        if (el && el.textContent) {
                            return el.textContent.trim();
                        }
                    }
                    return '';
                })()
                "#,
            )
            .await?
            .into_value::<String>()?;
        fund.fund_name = fund_name;

        let geographies = page
            .evaluate(
                r#"
                (() => {
                    // Define valid geographic regions - only actual location names
                    const validGeos = new Set([
                        'Global', 'Europe', 'Asia', 'Africa', 'America', 'Americas',
                        'North America', 'South America', 'Latin America',
                        'USA', 'US', 'United States', 'UK', 'United Kingdom', 
                        'Germany', 'France', 'Spain', 'Italy', 'Poland', 
                        'Ireland', 'Netherlands', 'Belgium', 'Switzerland', 
                        'Austria', 'Sweden', 'Norway', 'Denmark', 'Finland',
                        'Portugal', 'Greece', 'Czech Republic', 'Hungary',
                        'Romania', 'Bulgaria', 'Croatia', 'Serbia', 'Slovenia',
                        'Estonia', 'Latvia', 'Lithuania', 'Luxembourg',
                        'Canada', 'Mexico', 'Brazil', 'Argentina', 'Chile',
                        'China', 'Japan', 'India', 'Singapore', 'Australia',
                        'Israel', 'Turkey', 'Russia', 'Ukraine',
                        'EMEA', 'APAC', 'LATAM', 'NAMER', 'MENA', 
                        'CEE', 'DACH', 'Nordics', 'Benelux',
                        'Central Europe', 'Eastern Europe', 'Western Europe',
                        'Northern Europe', 'Southern Europe'
                    ]);
                    
                    const foundGeos = new Set();
                    
                    // Look for geography section specifically
                    const allElements = Array.from(document.querySelectorAll('*'));
                    
                    for (const el of allElements) {
                        const text = (el.textContent || '').trim();
                        
                        // Skip long text blocks and URLs
                        if (text.length > 200 || text.includes('http') || text.includes('www.')) {
                            continue;
                        }
                        
                        // Look for labeled geography sections
                        if (text.includes('Investment geography') || 
                            text.includes('Geography') || 
                            text.includes('Regions')) {
                            
                            // Split by common delimiters
                            const parts = text.split(/[,;:\/\n]/);
                            
                            for (const part of parts) {
                                const cleaned = part.trim();
                                
                                // Check if it's a valid geography
                                for (const geo of validGeos) {
                                    if (cleaned === geo || 
                                        (cleaned.toLowerCase() === geo.toLowerCase())) {
                                        foundGeos.add(geo);
                                    }
                                }
                            }
                        }
                    }
                    
                    // Also check for standalone geography mentions in small text blocks
                    for (const el of allElements) {
                        const text = (el.textContent || '').trim();
                        
                        // Only check small text blocks
                        if (text.length > 50 || text.includes('http')) {
                            continue;
                        }
                        
                        // Direct match with valid geographies
                        for (const geo of validGeos) {
                            if (text === geo || 
                                (text.toLowerCase() === geo.toLowerCase() && text.length === geo.length)) {
                                foundGeos.add(geo);
                            }
                        }
                    }
                    
                    // Return unique geographies, excluding any with special characters or URLs
                    return Array.from(foundGeos)
                        .filter(g => !g.includes('/') && !g.includes('.') && !g.includes('Type'))
                        .join(', ');
                })()
                "#,
            )
            .await?
            .into_value::<String>()?;
        fund.investment_geographies = geographies;

        // Extract AUM and convert to US number format
        let aum = page
            .evaluate(
                r#"
                (() => {
                    // Look for AUM in various formats
                    const texts = Array.from(document.querySelectorAll('*')).map(el => el.textContent || '');
                    const aumPatterns = [
                        /AUM[:\s]*([€$£¥]?\s*[\d,.\+]+(?:[.,]\d+)?\+?\s*[TBMK](?:rillion|illion)?\s*(?:EUR|USD|GBP)?)/i,
                        /Assets\s*Under\s*Management[:\s]*([€$£¥]?\s*[\d,.\+]+(?:[.,]\d+)?\+?\s*[TBMK](?:rillion|illion)?\s*(?:EUR|USD|GBP)?)/i
                    ];
                    
                    for (const text of texts) {
                        for (const pattern of aumPatterns) {
                            const match = text.match(pattern);
                            if (match && match[1]) {
                                let aumValue = match[1].trim();
                                
                                // Remove + sign if present
                                aumValue = aumValue.replace(/\+/g, '');
                                
                                // Parse the number and convert to euros
                                // Remove currency symbols and text
                                aumValue = aumValue.replace(/[€$£¥]/g, '').replace(/EUR|USD|GBP/gi, '').trim();
                                
                                // Determine the multiplier (convert to base euros)
                                let multiplier = 1;
                                if (aumValue.toLowerCase().includes('t')) {
                                    multiplier = 1000000000000; // trillion
                                    aumValue = aumValue.replace(/t(?:rillion)?/gi, '');
                                } else if (aumValue.toLowerCase().includes('b')) {
                                    multiplier = 1000000000; // billion
                                    aumValue = aumValue.replace(/b(?:illion)?/gi, '');
                                } else if (aumValue.toLowerCase().includes('m')) {
                                    multiplier = 1000000; // million
                                    aumValue = aumValue.replace(/m(?:illion)?/gi, '');
                                } else if (aumValue.toLowerCase().includes('k')) {
                                    multiplier = 1000; // thousand
                                    aumValue = aumValue.replace(/k/gi, '');
                                }
                                
                                // Convert European format (comma as decimal) to US format (period as decimal)
                                // First check if we have both comma and period
                                if (aumValue.includes(',') && aumValue.includes('.')) {
                                    // Determine which is the decimal separator
                                    const lastComma = aumValue.lastIndexOf(',');
                                    const lastPeriod = aumValue.lastIndexOf('.');
                                    if (lastComma > lastPeriod) {
                                        // European format: 1.000.000,50
                                        aumValue = aumValue.replace(/\./g, '').replace(',', '.');
                                    } else {
                                        // US format: 1,000,000.50
                                        aumValue = aumValue.replace(/,/g, '');
                                    }
                                } else if (aumValue.includes(',')) {
                                    // Check if comma is being used as decimal separator (European)
                                    const parts = aumValue.split(',');
                                    if (parts.length === 2 && parts[1].length <= 3) {
                                        // Likely European decimal: 3,8 or 100,5 or 3,1
                                        aumValue = aumValue.replace(',', '.');
                                    } else {
                                        // Likely thousands separator: 1,000
                                        aumValue = aumValue.replace(/,/g, '');
                                    }
                                }
                                
                                // Parse the number
                                const numValue = parseFloat(aumValue);
                                if (!isNaN(numValue)) {
                                    const finalValue = numValue * multiplier;
                                    // Return as clean number in euros (rounded to avoid decimals)
                                    return Math.round(finalValue).toString();
                                }
                            }
                        }
                    }
                    
                    return '';
                })()
                "#,
            )
            .await?
            .into_value::<String>()?;
        fund.aum = aum;

        // Extract LinkedIn URL
        let linkedin_url = page
            .evaluate(
                r#"
                (() => {
                    // Find LinkedIn links
                    const links = Array.from(document.querySelectorAll('a[href*="linkedin.com"]'));
                    for (const link of links) {
                        const href = link.href || '';
                        if (href.includes('linkedin.com/company/') || href.includes('linkedin.com/in/')) {
                            return href;
                        }
                    }
                    
                    // Check for LinkedIn in social media sections
                    const socialLinks = Array.from(document.querySelectorAll('[class*="social"] a, [class*="Social"] a, footer a'));
                    for (const link of socialLinks) {
                        const href = link.href || '';
                        if (href.includes('linkedin.com')) {
                            return href;
                        }
                    }
                    
                    // Check for LinkedIn icon links
                    const iconLinks = Array.from(document.querySelectorAll('a[aria-label*="LinkedIn"], a[title*="LinkedIn"]'));
                    for (const link of iconLinks) {
                        const href = link.href || '';
                        if (href) {
                            return href;
                        }
                    }
                    
                    return '';
                })()
                "#,
            )
            .await?
            .into_value::<String>()?;
        fund.linkedin_url = linkedin_url;

        let description = page
            .evaluate(
                r#"
                (() => {
                    // Define the boilerplate disclaimer text to exclude
                    const boilerplateText = "The material presented via this website is for informational purposes only. Nothing in this website constitutes a solicitation for the purchase or sale of any financial product or service. Material presented on this website does not constitute a public offering of securities or investment management services in any jurisdiction. Investing in startup and early stage companies involves risks, including loss of capital, illiquidity, lack of dividends and dilution, and it should be done only as part of a diversified portfolio. The Investments presented in this website are suitable only for investors who are sufficiently sophisticated to understand these risks and make their own investment decisions.";
                    
                    const selectors = ['.description', '.about', '.overview', '[class*="description"]', '[class*="about"]'];
                    for (const selector of selectors) {
                        const el = document.querySelector(selector);
                        if (el && el.textContent && el.textContent.length > 50) {
                            let text = el.textContent.trim().replace(/\n+/g, ' ').replace(/\s+/g, ' ');
                            // Remove boilerplate if present
                            if (text.includes(boilerplateText)) {
                                text = text.replace(boilerplateText, '').trim();
                            }
                            // Also check for partial boilerplate
                            if (text.includes("The material presented via this website is for informational purposes only")) {
                                const idx = text.indexOf("The material presented via this website");
                                text = text.substring(0, idx).trim();
                            }
                            if (text.length > 20) {
                                return text;
                            }
                        }
                    }
                    const paragraphs = Array.from(document.querySelectorAll('p'))
                        .filter(p => {
                            const text = p.textContent;
                            return text && 
                                   text.length > 100 && 
                                   !text.includes("The material presented via this website");
                        })
                        .map(p => p.textContent.trim())
                        .join(' ');
                    if (paragraphs) {
                        let cleanText = paragraphs.substring(0, 1000).replace(/\n+/g, ' ').replace(/\s+/g, ' ');
                        // Final check to remove any remaining boilerplate
                        if (cleanText.includes("The material presented via this website")) {
                            const idx = cleanText.indexOf("The material presented via this website");
                            cleanText = cleanText.substring(0, idx).trim();
                        }
                        return cleanText;
                    }
                    return '';
                })()
                "#,
            )
            .await?
            .into_value::<String>()?;
        fund.fund_description = description;

        let portfolio = page
            .evaluate(
                r#"
                (() => {
                    const portfolioCompanies = new Set();
                    
                    // First, look for text that contains "Portfolio" followed by company names
                    const allElements = Array.from(document.querySelectorAll('*'));
                    for (const el of allElements) {
                        const text = el.textContent || '';
                        
                        // Check for pattern like "Portfolio: Company1, Company2" or "Portfolio Company1; Company2"
                        if (text.includes('Portfolio') && !text.includes('portfolio management')) {
                            // Extract text after "Portfolio" keyword
                            const portfolioMatch = text.match(/Portfolio[:\s]+([^;]*(?:;[^;]*)*)/i);
                            if (portfolioMatch && portfolioMatch[1]) {
                                const companies = portfolioMatch[1]
                                    .split(/[,;]/)
                                    .map(c => c.trim())
                                    .filter(c => {
                                        // Filter out non-company text
                                        return c.length > 2 && 
                                               c.length < 100 && 
                                               !c.toLowerCase().includes('cookies') &&
                                               !c.toLowerCase().includes('material presented') &&
                                               !c.toLowerCase().includes('website') &&
                                               !c.toLowerCase().includes('aum') &&
                                               (c.includes('Ventures') || 
                                                c.includes('Capital') || 
                                                c.includes('Partners') ||
                                                c.includes('Fund') ||
                                                c.includes('Labs') ||
                                                c.includes('Accelerator'));
                                    });
                                companies.forEach(c => portfolioCompanies.add(c));
                            }
                        }
                    }
                    
                    // Also try to find portfolio sections with headers
                    const portfolioSection = allElements.find(el => {
                        const text = el.textContent || '';
                        return text.toLowerCase().includes('portfolio') && 
                               (el.tagName === 'H2' || el.tagName === 'H3' || el.tagName === 'H4');
                    });
                    
                    if (portfolioSection) {
                        let sibling = portfolioSection.nextElementSibling;
                        let count = 0;
                        while (sibling && count < 5) {  // Limit to next 5 siblings
                            const items = sibling.querySelectorAll('li, a, span');
                            items.forEach(item => {
                                const text = item.textContent ? item.textContent.trim() : '';
                                if (text && text.length > 2 && text.length < 100 &&
                                    (text.includes('Ventures') || 
                                     text.includes('Capital') || 
                                     text.includes('Partners') ||
                                     text.includes('Fund') ||
                                     text.includes('Labs'))) {
                                    portfolioCompanies.add(text);
                                }
                            });
                            sibling = sibling.nextElementSibling;
                            count++;
                        }
                    }
                    
                    // Filter out any remaining noise
                    const cleanPortfolio = Array.from(portfolioCompanies)
                        .filter(company => {
                            const lower = company.toLowerCase();
                            return !lower.includes('investing in startup') &&
                                   !lower.includes('material presented') &&
                                   !lower.includes('cookies') &&
                                   !lower.includes('website');
                        });
                    
                    return cleanPortfolio.join('; ');
                })()
                "#,
            )
            .await?
            .into_value::<String>()?;
        fund.fund_portfolio = portfolio;

        Ok(fund)
    }

    pub async fn close(mut self) -> Result<()> {
        self.browser.close().await?;
        Ok(())
    }
}

pub async fn scrape_with_retry(scraper: &VestbeeScraper, url: &str, max_retries: u32) -> Result<Fund> {
    let mut retries = 0;
    let mut delay = Duration::from_secs(2);

    loop {
        match scraper.scrape_fund_details(url).await {
            Ok(fund) => return Ok(fund),
            Err(e) if retries < max_retries => {
                warn!("Attempt {} failed for {}: {:?}, retrying in {:?}", retries + 1, url, e, delay);
                tokio::time::sleep(delay).await;
                delay *= 2;
                retries += 1;
            }
            Err(e) => {
                error!("Failed to scrape {} after {} retries: {:?}", url, max_retries, e);
                return Err(e);
            }
        }
    }
}