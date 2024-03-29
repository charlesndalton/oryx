use anyhow::{Context, Result};
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use derive_getters::Getters;
use derive_new::new;
use log::{error, info};
use std::env;
use std::str::FromStr;
use std::sync::Arc;

const STARGATE_STRATEGY_ADDRESSES: [&'static str; 2] = [
    "0x7C85c0a8E2a45EefF98A10b6037f70daf714B7cf",
    "0xeAD650E673F497CdBE365F7a855273BbB468e454",
];

#[derive(new, Getters, Debug)]
pub struct StargateReport {
    individual_strategy_reports: Vec<IndividualStrategyReport>,
}

#[derive(new, Getters, Debug)]
pub struct IndividualStrategyReport {
    asset_name: String,
    strategy_tvl: BigDecimal,
    pool_liquidity: BigDecimal,
    pool_liabilities: BigDecimal,
    current_ratio: BigDecimal,
}

#[tokio::main]
async fn main() -> Result<()> {
    let telegram_token = env::var("ORYX_TELEGRAM_TOKEN").expect("ORYX_TELEGRAM_TOKEN not set");
    let infura_api_key = env::var("INFURA_API_KEY").expect("INFURA_API_KEY not set");

    env_logger::init();

    info!("================== ORYX RUNNING ==================");

    let report = report_creator::create_report(infura_api_key)
        .await
        .context("Failed to create report")?;

    report_publisher::publish_report(report, telegram_token)
        .await
        .context("Failed to publish report")?;

    Ok(())
}

mod report_publisher {
    use super::*;
    use num_format::{Locale, ToFormattedString};

    pub async fn publish_report(report: StargateReport, telegram_token: String) -> Result<()> {
        let mut report_formatted = vec![String::from("Daily Stargate Report")];

        for individual_strategy_report in report.individual_strategy_reports() {
            report_formatted.push(String::from("--------------"));
            report_formatted.push(format!(
                "Asset – {}",
                &individual_strategy_report.asset_name()
            ));
            report_formatted.push(format!(
                "Yearn Strategy TVL: ${}",
                prettify(&individual_strategy_report.strategy_tvl())
            ));
            report_formatted.push(format!(
                "Pool Current Ratio: {}",
                individual_strategy_report.current_ratio().with_scale(2)
            ));
            report_formatted.push(format!(
                "Total Pool Liabilities: ${}",
                prettify(&individual_strategy_report.pool_liabilities())
            ));
            report_formatted.push(format!(
                "Total Pool Liquidity: ${}",
                prettify(&individual_strategy_report.pool_liquidity())
            ));
        }

        let report_for_telegram = report_formatted.join("\n");

        info!("Report: {:?}", report_for_telegram);

        telegram_client::send_message_to_committee(&report_for_telegram, &telegram_token).await?;

        Ok(())
    }

    fn prettify(num: &BigDecimal) -> String {
        // unwrap is ok because we shouldn't have a number larger than 2^128
        num.to_u128().unwrap().to_formatted_string(&Locale::en)
    }
}

mod telegram_client {
    use super::*;
    use urlencoding::encode;

    const STARGATE_COMMITTEE_TELEGRAM_CHAT_ID: i64 = -753837580;

    pub async fn send_message_to_committee(message: &str, token: &str) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage?chat_id={}&text={}",
            token,
            STARGATE_COMMITTEE_TELEGRAM_CHAT_ID,
            encode(message)
        );

        reqwest::get(url).await?;

        Ok(())
    }
}

mod report_creator {
    use super::*;
    use crate::blockchain_client::{StargateStrategy, IERC20};
    use ethers::abi::Address;

    pub async fn create_report(infura_api_key: String) -> Result<StargateReport> {
        let mut individual_strategy_reports = Vec::new();
        let eth_client = blockchain_client::create_client(&infura_api_key)?;

        for strategy_address in STARGATE_STRATEGY_ADDRESSES {
            let strategy = StargateStrategy::new(
                strategy_address.parse::<Address>()?,
                Arc::clone(&eth_client),
            );

            let want = strategy.get_want(Arc::clone(&eth_client)).await?;

            let symbol = want.symbol().await?;

            info!("Creating report for {}", symbol);

            let decimals = want.decimals().await?;

            info!("Strategy decimals: {}", decimals);

            let total_position =
                (strategy.get_total_position().await? / 10_i64.pow(decimals)).with_scale(0);

            info!("Total position: ${}", total_position);

            let liquidity_pool_address = strategy.get_underlying_liquidity_pool().await?;

            info!("Pool address: {}", liquidity_pool_address);

            let total_liquidity = (want.balance_of(liquidity_pool_address).await?
                / 10_i64.pow(decimals))
            .with_scale(0);

            info!("Liquidity: {}", total_liquidity);

            let pool_token = IERC20::new(liquidity_pool_address, Arc::clone(&eth_client));

            let pool_liabilities =
                (pool_token.total_supply().await? / 10_i64.pow(decimals)).with_scale(0);

            info!("Pool liabilities: {}", pool_liabilities);

            let current_ratio = &total_liquidity / &pool_liabilities;

            individual_strategy_reports.push(IndividualStrategyReport::new(
                symbol,
                total_position,
                total_liquidity,
                pool_liabilities,
                current_ratio,
            ));
        }

        Ok(StargateReport::new(individual_strategy_reports))
    }
}

mod blockchain_client {
    use super::*;
    use ethers::prelude::*;

    abigen!(UnwrappedIERC20, "./src/abis/IERC20.json");
    pub struct IERC20 {
        instance: UnwrappedIERC20<Provider<Http>>, // holds the ethers object so we don't need to re-create it a bunch
    }

    abigen!(
        UnwrappedStargateStrategy,
        "./src/abis/StargateStrategy.json"
    );
    pub struct StargateStrategy {
        instance: UnwrappedStargateStrategy<Provider<Http>>,
    }
    //pub type StargateStrategy = UnwrappedStargateStrategy<Provider::<Http>>;

    pub type Client = Arc<Provider<Http>>;

    pub fn create_client(infura_api_key: &str) -> Result<Client> {
        let infura_url = format!("https://mainnet.infura.io/v3/{}", infura_api_key);
        let client = Provider::<Http>::try_from(infura_url)?;
        Ok(Arc::new(client))
    }

    impl IERC20 {
        pub fn new(token_address: Address, client: Client) -> Self {
            Self {
                instance: UnwrappedIERC20::new(token_address, client),
            }
        }

        pub async fn decimals(&self) -> Result<u32> {
            let decimals: u32 = self.instance.decimals().call().await?.into();
            Ok(decimals)
        }

        pub async fn symbol(&self) -> Result<String> {
            Ok(self.instance.symbol().call().await?)
        }

        pub async fn balance_of(&self, address: Address) -> Result<BigDecimal> {
            let balance = self.instance.balance_of(address).call().await?;

            Ok(BigDecimal::from_u128(balance.as_u128()).unwrap())
        }

        pub async fn total_supply(&self) -> Result<BigDecimal> {
            let total_supply = self.instance.total_supply().call().await?;

            Ok(BigDecimal::from_u128(total_supply.as_u128()).unwrap())
        }
    }

    impl StargateStrategy {
        pub fn new(strategy_address: Address, client: Client) -> Self {
            Self {
                instance: UnwrappedStargateStrategy::new(strategy_address, client),
            }
        }

        pub async fn get_want(&self, client: Client) -> Result<IERC20> {
            let want_address = self.instance.want().call().await?;
            let want = IERC20::new(want_address, client);

            Ok(want)
        }

        pub async fn get_total_position(&self) -> Result<BigDecimal> {
            let total_position = self.instance.value_of_lp_tokens().call().await?;

            Ok(BigDecimal::from_u128(total_position.as_u128()).unwrap()) // would cause problems for numbers larger than 2^128
        }

        pub async fn get_underlying_liquidity_pool(&self) -> Result<Address> {
            Ok(self.instance.liquidity_pool().call().await?)
        }
    }
}
