use anyhow::Result;
use rust_xlsxwriter::{Format, Workbook};
use crate::models::Fund;

pub struct ExcelExporter {
    workbook: Workbook,
}

impl ExcelExporter {
    pub fn new() -> Result<Self> {
        Ok(Self {
            workbook: Workbook::new(),
        })
    }
    
    pub fn write_funds(&mut self, funds: &[Fund]) -> Result<()> {
        let worksheet = self.workbook.add_worksheet();
        
        // Create header format: navy background, bold, white font
        let header_format = Format::new()
            .set_bold()
            .set_background_color(rust_xlsxwriter::Color::RGB(0x000080)) // Navy blue
            .set_font_color(rust_xlsxwriter::Color::White)
            .set_border(rust_xlsxwriter::FormatBorder::Thin);
        
        // Write headers
        let headers = [
            "Fund Name",
            "Fund URL", 
            "AUM (€)",
            "LinkedIn URL",
            "Investment Geographies",
            "Fund Description",
            "Fund Portfolio",
        ];
        
        for (col, header) in headers.iter().enumerate() {
            worksheet.write_with_format(0, col as u16, *header, &header_format)?;
        }
        
        // Set column widths for better readability
        worksheet.set_column_width(0, 30)?;  // Fund Name
        worksheet.set_column_width(1, 50)?;  // Fund URL
        worksheet.set_column_width(2, 15)?;  // AUM
        worksheet.set_column_width(3, 40)?;  // LinkedIn URL
        worksheet.set_column_width(4, 30)?;  // Geographies
        worksheet.set_column_width(5, 60)?;  // Description
        worksheet.set_column_width(6, 50)?;  // Portfolio
        
        // Freeze the header row
        worksheet.set_freeze_panes(1, 0)?;
        
        // Regular cell format with borders
        let cell_format = Format::new()
            .set_border(rust_xlsxwriter::FormatBorder::Thin);
        
        // Money format for AUM (euros with thousand separator, no decimals)
        let money_format = Format::new()
            .set_border(rust_xlsxwriter::FormatBorder::Thin)
            .set_num_format("#,##0");
        
        // Write all funds
        for (row_idx, fund) in funds.iter().enumerate() {
            let row = (row_idx + 1) as u32;  // +1 for header
            
            worksheet.write_with_format(row, 0, &fund.fund_name, &cell_format)?;
            worksheet.write_with_format(row, 1, &fund.fund_url, &cell_format)?;
            
            // Write AUM as number if available
            if !fund.aum.is_empty() {
                if let Ok(aum_value) = fund.aum.parse::<f64>() {
                    worksheet.write_with_format(row, 2, aum_value, &money_format)?;
                } else {
                    worksheet.write_with_format(row, 2, &fund.aum, &cell_format)?;
                }
            } else {
                worksheet.write_with_format(row, 2, "", &cell_format)?;
            }
            
            worksheet.write_with_format(row, 3, &fund.linkedin_url, &cell_format)?;
            worksheet.write_with_format(row, 4, &fund.investment_geographies, &cell_format)?;
            worksheet.write_with_format(row, 5, &fund.fund_description, &cell_format)?;
            worksheet.write_with_format(row, 6, &fund.fund_portfolio, &cell_format)?;
        }
        
        Ok(())
    }
    
    pub fn save(mut self, filename: &str) -> Result<()> {
        self.workbook.save(filename)?;
        Ok(())
    }
}