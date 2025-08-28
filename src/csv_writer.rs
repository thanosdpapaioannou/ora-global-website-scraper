use anyhow::Result;
use csv::Writer;
use std::fs::File;

use crate::models::Fund;

pub struct CsvExporter {
    writer: Writer<File>,
}

impl CsvExporter {
    pub fn new(filename: &str) -> Result<Self> {
        let writer = Writer::from_path(filename)?;
        Ok(Self { writer })
    }

    pub fn write_header(&mut self) -> Result<()> {
        self.writer.write_record(&[
            "fund_name",
            "fund_url",
            "AUM (€)",
            "linkedin_url",
            "investment_geographies",
            "fund_description",
            "fund_portfolio",
        ])?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn write_fund(&mut self, fund: &Fund) -> Result<()> {
        self.writer.write_record(&[
            &fund.fund_name,
            &fund.fund_url,
            &fund.aum,
            &fund.linkedin_url,
            &fund.investment_geographies,
            &fund.fund_description,
            &fund.fund_portfolio,
        ])?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn finalize(mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}