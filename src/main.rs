use std::env;
use anyhow::{Result, Context};

fn main() -> Result<()> {
    let telegram_token = env::var("ORYX_TELEGRAM_TOKEN").expect("ORYX_TELEGRAM_TOKEN not set");
    let infura_api_key = env::var("INFURA_API_KEY").expect("INFURA_API_KEY not set");

    report_creator::create_report()?;

    report_publisher::publish_report()?;

    Ok(())
}

mod report_creator {
    use super::*;

    pub fn create_report() -> Result<()> {
        Ok(())
    }
}

mod report_publisher {
    use super::*;

    pub fn publish_report() -> Result<()> {
        Ok(())
    }
}
