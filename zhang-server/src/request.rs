use bigdecimal::BigDecimal;
use chrono::{DateTime, Datelike, Local, Utc};
use serde::Deserialize;
use std::cmp::max;
use std::collections::HashSet;
use zhang_ast::Flag;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum AccountBalanceRequest {
    Check { account_name: String, amount: AmountRequest },
    Pad { account_name: String, amount: AmountRequest, pad: String },
}

#[derive(Deserialize)]
pub struct FileUpdateRequest {
    pub content: String,
}

#[derive(Deserialize)]
pub enum StatisticInterval {
    Day,
    Week,
    Month,
}

#[derive(Deserialize)]
pub struct StatisticRequest {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}
#[derive(Deserialize)]
pub struct StatisticGraphRequest {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub interval: StatisticInterval,
}

#[derive(Deserialize)]
pub struct ReportRequest {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct JournalRequest {
    pub page: Option<u32>,
    pub size: Option<u32>,
    pub keyword: Option<String>,
    pub tags: Option<HashSet<String>>,
    pub links: Option<HashSet<String>>,
}
impl JournalRequest {
    pub fn page(&self) -> u32 {
        max(self.page.unwrap_or(1), 1)
    }
    pub fn offset(&self) -> u32 {
        let page = self.page();
        (page - 1) * self.limit()
    }
    pub fn limit(&self) -> u32 {
        self.size.unwrap_or(100)
    }
}

#[derive(Deserialize)]
pub struct CreateTransactionRequest {
    pub datetime: DateTime<Utc>,
    pub payee: String,
    pub flag: Option<FlagRequest>,
    pub narration: Option<String>,
    pub postings: Vec<CreateTransactionPostingRequest>,
    pub metas: Vec<MetaRequest>,
    pub tags: Vec<String>,
    pub links: Vec<String>,
}

#[derive(Deserialize)]
pub enum FlagRequest {
    Okay,
    Warning,
    BalancePad,
    BalanceCheck,
    #[serde(untagged)]
    Custom(char),
}

impl From<FlagRequest> for Flag {
    fn from(req: FlagRequest) -> Self {
        match req {
            FlagRequest::Okay => Flag::Okay,
            FlagRequest::Warning => Flag::Warning,
            FlagRequest::BalancePad => Flag::BalancePad,
            FlagRequest::BalanceCheck => Flag::BalanceCheck,
            FlagRequest::Custom(c) => Flag::Custom(c.to_string()),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateTransactionPostingRequest {
    pub account: String,
    pub unit: Option<AmountRequest>,
}

#[derive(Deserialize)]
pub struct AmountRequest {
    pub number: BigDecimal,
    pub commodity: String,
}

#[derive(Deserialize)]
pub struct MetaRequest {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct BudgetListRequest {
    pub month: Option<u32>,
    pub year: Option<u32>,
}
impl BudgetListRequest {
    pub fn as_interval(&self) -> u32 {
        let time = Local::now();
        self.year.unwrap_or(time.year() as u32) * 100 + self.month.unwrap_or(time.month())
    }
}
