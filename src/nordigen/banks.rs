// nordigen-cli: A simple Nordigen client
// Copyright (C) 2022  Joao Eduardo Luis <joao@abysmo.io>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
use chrono::{DateTime, Days, Utc};
use serde::{Deserialize, Serialize};

use crate::nordigen::auth_http_cb;

#[derive(Deserialize)]
pub struct BankEntry {
    pub id: String,
    pub name: String,
    pub bic: String,
    pub transaction_total_days: String,
    pub countries: Vec<String>,
    pub logo: String,
}

#[derive(Serialize)]
struct BankRequisitionRequest {
    redirect: String,
    institution_id: String,
    user_language: String,
}

#[derive(Deserialize)]
struct BankRequisitionReply {
    id: String,
    created: DateTime<Utc>,
    // redirect: String,
    // status: String,
    // agreement: String,
    // accounts: Vec<String>,
    // reference: String,
    // user_language: String,
    link: String,
    // ssn: Option<String>,
    // account_selection: bool,
    // redirect_immediate: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BankRequisitionState {
    pub requisition_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct BankRequisitionsGetReply {
    // id: String,
    // created: String,
    // institution_id: String,
    accounts: Vec<String>,
    // account_selection: bool,
}

#[derive(Serialize, Deserialize)]
pub struct BankAuthState {
    pub bank_id: String,
    pub requisition: BankRequisitionState,
}

#[derive(Deserialize)]
struct AccountInfo {
    pub id: String,
    pub created: Option<DateTime<Utc>>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub iban: String,
    pub institution_id: String,
    // pub owner_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountDetails {
    pub currency: String,
    pub owner_name: Option<String>,
    pub name: Option<String>,
    pub product: Option<String>,
    pub cash_account_type: Option<String>,
}

#[derive(Deserialize)]
struct AccountDetailsReply {
    account: AccountDetails,
}

pub struct AccountMeta {
    pub id: String,
    pub created_at: Option<DateTime<Utc>>,
    pub accessed_at: Option<DateTime<Utc>>,
    pub iban: String,
    pub institution_id: String,
    pub owner_name: Option<String>,
    pub name: Option<String>,
    pub currency: String,
    pub product: Option<String>,
    pub account_type: Option<String>,
}

#[derive(Deserialize)]
pub struct TransactionDebtorAccount {
    pub iban: String,
}

#[derive(Deserialize)]
pub struct TransactionAmount {
    pub currency: String,
    pub amount: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBookedTransaction {
    pub transaction_id: String,
    pub debtor_name: Option<String>,
    pub debtor_account: Option<String>,
    pub transaction_amount: TransactionAmount,
    pub bank_transaction_code: Option<String>,
    pub booking_date: String,
    pub value_date: String,
    pub remittance_information_unstructured: Option<String>,
    pub internal_transaction_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountPendingTransaction {
    pub transaction_amount: TransactionAmount,
    pub value_date: String,
    pub remittance_information_unstructured: Option<String>,
}

#[derive(Deserialize)]
pub struct AccountTransactions {
    pub booked: Vec<AccountBookedTransaction>,
    pub pending: Vec<AccountPendingTransaction>,
}

#[derive(Deserialize)]
struct AccountTransactionsReply {
    pub transactions: AccountTransactions,
}

pub async fn list(
    token: &String,
    country: &Option<String>,
) -> Result<Vec<BankEntry>, String> {
    let client = reqwest::Client::new();
    let mut req = client
        .get("https://ob.nordigen.com/api/v2/institutions/")
        .header("accept", "application/json")
        .bearer_auth(token);

    if let Some(ccode) = country {
        req = req.query(&[("country", ccode)]);
    }
    let res = match req.send().await {
        Err(error) => {
            return Err(format!("Unable to obtain bank list: {}", error));
        }
        Ok(res) => res,
    };

    let banks: Vec<BankEntry> = match res.json::<Vec<BankEntry>>().await {
        Err(error) => {
            return Err(format!("Unable to parse bank list: {}", error));
        }
        Ok(res) => res,
    };

    Ok(banks)
}

pub struct Authorize {
    token: String,
    bank_id: String,
    requisition: Option<BankRequisitionReply>,
}

impl Authorize {
    pub fn new(token: &String, bank_id: &String) -> Authorize {
        Authorize {
            token: token.clone(),
            bank_id: bank_id.clone(),
            requisition: None,
        }
    }

    pub async fn start(self: &mut Self) -> Result<String, String> {
        let client = reqwest::Client::new();
        let res = match client
            .post("https://ob.nordigen.com/api/v2/requisitions/")
            .header("accept", "application/json")
            .header("Content-Type", "application/json")
            .bearer_auth(&self.token)
            .json(&BankRequisitionRequest {
                redirect: String::from("http://127.0.0.1:1337"),
                institution_id: self.bank_id.clone(),
                user_language: String::from("EN"),
            })
            .send()
            .await
        {
            Err(error) => {
                return Err(format!(
                    "Error obtaining authorization: {}",
                    error
                ));
            }
            Ok(res) => res,
        };

        let requisition = res
            .json::<BankRequisitionReply>()
            .await
            .unwrap_or_else(|err| {
                eprintln!("Error obtaining requisition response: {err}");
                std::process::exit(1);
            });
        let link = requisition.link.clone();
        self.requisition = Some(requisition);

        Ok(link)
    }

    pub async fn wait_callback(
        self: &mut Self,
    ) -> Result<BankRequisitionState, String> {
        let req = match &self.requisition {
            None => {
                return Err(String::from(
                    "Unable to find existing requisition!",
                ));
            }
            Some(res) => res,
        };

        let bank_ref = match auth_http_cb::wait_for_response() {
            Err(err) => {
                return Err(format!(
                    "Unable to obtain bank's callback: {}",
                    err
                ));
            }
            Ok(res) => res,
        };

        if req.id != bank_ref {
            return Err(format!(
                "Mismatch between requisition id and bank's callback: {}",
                bank_ref
            ));
        }

        Ok(BankRequisitionState {
            requisition_id: req.id.clone(),
            created_at: req.created,
        })
    }
}

impl BankAuthState {
    pub fn new(
        bank_id: &String,
        requisition: &BankRequisitionState,
    ) -> BankAuthState {
        BankAuthState {
            bank_id: bank_id.clone(),
            requisition: requisition.clone(),
        }
    }

    pub fn parse(path: &std::path::PathBuf) -> Result<Self, String> {
        if !path.exists() {
            return Err(format!(
                "Bank state file at {} does not exist!",
                path.display()
            ));
        }

        let contents = match std::fs::read_to_string(path) {
            Err(err) => {
                return Err(format!(
                    "Error reading file at {}: {}",
                    path.display(),
                    err
                ));
            }
            Ok(value) => value,
        };

        let state: BankAuthState = match serde_json::from_str(&contents) {
            Err(err) => {
                return Err(format!(
                    "Unable to parse bank state file at {}: {}",
                    path.display(),
                    err
                ));
            }
            Ok(value) => value,
        };
        Ok(state)
    }

    pub fn write(
        self: &Self,
        path: &std::path::PathBuf,
    ) -> Result<&Self, String> {
        let buffer = match std::fs::File::create(path) {
            Err(err) => {
                return Err(format!(
                    "Unable to open bank state file for writing: {}",
                    err
                ));
            }
            Ok(res) => res,
        };

        match serde_json::to_writer_pretty(buffer, &self) {
            Err(err) => {
                return Err(format!(
                    "Unable to write bank state to disk: {}",
                    err
                ));
            }
            Ok(_) => {}
        };
        Ok(self)
    }
}

pub struct Accounts {
    token: String,
    requisition_id: String,
}

impl Accounts {
    pub fn new(token: &String, req_id: &String) -> Accounts {
        Accounts {
            token: token.clone(),
            requisition_id: req_id.clone(),
        }
    }

    pub async fn list(self: &Self) -> Result<Vec<String>, String> {
        let client = reqwest::Client::new();
        let req = match client
            .get(format!(
                "https://ob.nordigen.com/api/v2/requisitions/{}/",
                self.requisition_id
            ))
            .header("accept", "application/json")
            .bearer_auth(&self.token)
            .send()
            .await
        {
            Err(err) => {
                return Err(format!("Error listing accounts: {}", err));
            }
            Ok(res) => res,
        };

        let requisition = req
            .json::<BankRequisitionsGetReply>()
            .await
            .unwrap_or_else(|err| {
                eprintln!("Error obtaining requisition response: {}", err);
                std::process::exit(1);
            });

        Ok(requisition.accounts)
    }

    async fn info(
        self: &Self,
        account_id: &String,
    ) -> Result<AccountInfo, String> {
        let client = reqwest::Client::new();
        let req = match client
            .get(format!(
                "https://ob.nordigen.com/api/v2/accounts/{}/",
                account_id
            ))
            .header("accept", "application/json")
            .bearer_auth(&self.token)
            .send()
            .await
        {
            Err(err) => {
                return Err(format!("Error obtaining account info: {}", err));
            }
            Ok(res) => res,
        };

        let contents = req.json::<AccountInfo>().await.unwrap_or_else(|err| {
            eprintln!("Error obtaining account info response: {}", err);
            std::process::exit(1);
        });

        Ok(contents)
    }

    async fn details(
        self: &Self,
        account_id: &String,
    ) -> Result<AccountDetails, String> {
        let client = reqwest::Client::new();
        let req = match client
            .get(format!(
                "https://ob.nordigen.com/api/v2/accounts/{}/details/",
                account_id
            ))
            .header("accept", "application/json")
            .bearer_auth(&self.token)
            .send()
            .await
        {
            Err(err) => {
                return Err(format!(
                    "Error obtaining account details: {}",
                    err
                ));
            }
            Ok(res) => res,
        };

        let contents =
            req.json::<AccountDetailsReply>()
                .await
                .unwrap_or_else(|err| {
                    eprintln!(
                        "Error obtaining account details response: {}",
                        err
                    );
                    std::process::exit(1);
                });

        Ok(contents.account)
    }

    pub async fn meta(
        self: &Self,
        account_id: &String,
    ) -> Result<AccountMeta, String> {
        let info = match self.info(account_id).await {
            Err(err) => {
                return Err(format!(
                    "Error obtaining account metadata: {}",
                    err
                ));
            }
            Ok(res) => res,
        };

        let details = match self.details(account_id).await {
            Err(err) => {
                return Err(format!(
                    "Error obtaining account metadata: {}",
                    err
                ));
            }
            Ok(res) => res,
        };

        Ok(AccountMeta {
            id: info.id,
            created_at: info.created,
            accessed_at: info.last_accessed,
            iban: info.iban,
            institution_id: info.institution_id,
            owner_name: details.owner_name,
            name: details.name,
            currency: details.currency,
            product: details.product,
            account_type: details.cash_account_type,
        })
    }

    pub async fn meta_all(self: &Self) -> Result<Vec<AccountMeta>, String> {
        let mut all: Vec<AccountMeta> = Vec::new();

        let acclst = match self.list().await {
            Err(err) => {
                return Err(err);
            }
            Ok(res) => res,
        };

        for account_id in &acclst {
            let meta = match self.meta(account_id).await {
                Err(err) => {
                    return Err(err);
                }
                Ok(res) => res,
            };
            all.push(meta);
        }

        Ok(all)
    }

    pub async fn transactions(
        self: &Self,
        account_id: &String,
    ) -> Result<AccountTransactions, String> {
        let now = Utc::now();
        let then = now.clone().checked_sub_days(Days::new(30)).unwrap();

        let start = then.format("%Y-%m-%d").to_string();
        let end = now.format("%Y-%m-%d").to_string();

        let client = reqwest::Client::new();
        let req = match client
            .get(format!(
                "https://ob.nordigen.com/api/v2/accounts/{}/transactions/",
                account_id
            ))
            .query(&[("date_from", start), ("date_to", end)])
            .header("accept", "application/json")
            .bearer_auth(&self.token)
            .send()
            .await
        {
            Err(err) => {
                return Err(format!("Error obtaining transactions: {}", err));
            }
            Ok(res) => res,
        };

        let contents = req
            .json::<AccountTransactionsReply>()
            .await
            .unwrap_or_else(|err| {
                eprintln!(
                    "Error obtaining transaction details response: {}",
                    err
                );
                std::process::exit(1);
            });
        Ok(contents.transactions)
    }

    pub async fn balance(self: &Self, account_id: &String) {
        let client = reqwest::Client::new();
        let req = match client
            .get(format!(
                "https://ob.nordigen.com/api/v2/accounts/{}/balances/",
                account_id
            ))
            .header("accept", "application/json")
            .bearer_auth(&self.token)
            .send()
            .await
        {
            Err(err) => {
                println!("Error obtaining account balance: {}", err);
                std::process::exit(1);
            }
            Ok(res) => res,
        };

        let tmp = req.text().await.unwrap_or_else(|err| {
            eprintln!("kaboom: {}", err);
            std::process::exit(1);
        });
        println!("{}", tmp);
    }
}
