use std::env;
use anyhow::{Result, Context};
use bigdecimal::BigDecimal;
use derive_getters::{Getters};
use derive_new::{new};

#[derive(new, Getters)]
pub struct StargateReport {
    individual_strategy_reports: Vec<IndividualStrategyReport>
}

#[derive(new, Getters)]
pub struct IndividualStrategyReport {
    strategy_tvl: BigDecimal,
    pool_liquidity: BigDecimal,
}

fn main() -> Result<()> {
    let telegram_token = env::var("ORYX_TELEGRAM_TOKEN").expect("ORYX_TELEGRAM_TOKEN not set");
    let infura_api_key = env::var("INFURA_API_KEY").expect("INFURA_API_KEY not set");

    let report = report_creator::create_report()?;

    report_publisher::publish_report(report)?;

    Ok(())
}

mod report_creator {
    use super::*;

    pub fn create_report() -> Result<StargateReport> {
        let mut individual_strategy_reports = Vec::new();

        Ok(StargateReport::new(individual_strategy_reports))
    }
}

mod report_publisher {
    use super::*;

    pub fn publish_report(report: StargateReport) -> Result<()> {
        Ok(())
    }
}


