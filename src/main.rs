use std::io::ErrorKind;

// nordigen-cli: A simple Nordigen client
// Copyright (C) 2022  Joao Eduardo Luis <joao@abysmo.io>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
use clap::Parser;

pub mod cli;
pub mod nordigen;

use cli::{
    AuthorizeCmd, BankAccountBalanceCmd, BankAuthorizeCmd, BankCmds,
    BankListCmd, Cli, Commands, RefreshCmd,
};
use cli::{BankAccountCmds, BankAccountTransactionsCmd};
use nordigen::authorize;
use nordigen::banks;
use nordigen::config::NordigenConfig;
use prettytable::{row, Attr, Cell, Row, Table};

use crate::nordigen::state::NordigenState;

fn get_state(path: &std::path::PathBuf) -> Result<NordigenState, ErrorKind> {
    if !path.exists() {
        return Err(ErrorKind::NotFound);
    }

    match NordigenState::parse(&path) {
        Err(error) => {
            eprintln!("Error obtaining on-disk state: {}", error);
            return Err(ErrorKind::InvalidData);
        }
        Ok(res) => {
            return Ok(res);
        }
    };
}

fn print_state_error(err: ErrorKind) {
    match err {
        ErrorKind::NotFound => {
            eprintln!("State file not found");
        }
        ErrorKind::InvalidData => {
            eprintln!("Invalid state file found");
        }
        _ => {
            eprintln!("Unknown error!");
        }
    }
}

fn get_state_or_exit(path: &std::path::PathBuf) -> NordigenState {
    match get_state(&path) {
        Err(error) => {
            print_state_error(error);
            std::process::exit(1);
        }
        Ok(res) => return res,
    };
}

async fn do_authorize(cmd: &AuthorizeCmd) {
    println!("authorize client");

    if cmd.state.exists() {
        println!("Found on-disk state...");
        let state = NordigenState::parse(&cmd.state).unwrap_or_else(|err| {
            eprintln!("Error obtaining on-disk state: {}", err);
            std::process::exit(1);
        });

        if state.is_refresh_expired() {
            eprintln!("Refresh token has expired!");
        } else if state.is_token_expired() {
            println!("Access token expired. Please refresh!");
            std::process::exit(0);
        } else {
            println!("Authorization still valid");
            std::process::exit(0);
        }
    }

    println!("Obtaining new authorization...");

    let config = NordigenConfig::parse(&cmd.config).unwrap_or_else(|err| {
        println!("Error parsing config: {err}");
        std::process::exit(1);
    });
    println!("config: {}", config);
    let authorization =
        authorize::authorize(&config).await.unwrap_or_else(|err| {
            println!("Error obtaining authorization: {err}");
            std::process::exit(1);
        });

    let state = NordigenState::write(
        &cmd.state,
        authorization.access,
        authorization.refresh,
        authorization.access_expires,
        authorization.refresh_expires,
    )
    .unwrap_or_else(|err| {
        eprintln!("Unable to write state: {err}");
        std::process::exit(1);
    });

    let access_expires = state.token_expires_on().to_string();
    println!(
        "Obtained authorization token; expires on {}",
        access_expires
    );
}

async fn do_refresh(cmd: &RefreshCmd) {
    println!("refresh authorization");
    let state = get_state_or_exit(&cmd.state);
    if !state.is_token_expired() {
        println!("Token is still valid and does not need to be refreshed.");
        std::process::exit(0);
    } else if state.is_refresh_expired() {
        eprintln!("Refresh token has expired. Please authorize again.");
        std::process::exit(1);
    }

    let (new_token, new_expires) = authorize::refresh(&state.refresh_token)
        .await
        .unwrap_or_else(|err| {
            eprintln!("Error refreshing token: {}", err);
            std::process::exit(1);
        });

    let new_state = NordigenState::write(
        &cmd.state,
        new_token,
        state.refresh_token,
        new_expires,
        state.refresh_expires,
    )
    .unwrap_or_else(|err| {
        eprintln!("Unable to write state: {}", err);
        std::process::exit(1);
    });

    let access_expires = new_state.token_expires_on().to_string();
    println!(
        "Successfully refreshed; new token expires on {}",
        access_expires
    );
}

async fn do_bank_list(cmd: &BankListCmd, statepath: &std::path::PathBuf) {
    let state = get_state_or_exit(&statepath);
    if state.is_token_expired() {
        eprintln!("Token has expired. Maybe refresh?");
        std::process::exit(1);
    }
    let banks = match banks::list(&state.token, &cmd.country).await {
        Err(error) => {
            eprintln!("Error obtaining bank list: {}", error);
            std::process::exit(1);
        }
        Ok(res) => res,
    };

    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("Country").with_style(Attr::Bold),
        Cell::new("ID").with_style(Attr::Bold),
        Cell::new("Name").with_style(Attr::Bold),
        Cell::new("Tx Days").with_style(Attr::Bold),
    ]));

    for bank in &banks {
        let country_str = bank.countries.join(", ");
        table.add_row(row![
            country_str,
            bank.id,
            bank.name,
            bank.transaction_total_days
        ]);
    }
    table.printstd();
}

async fn do_bank_authorization(
    cmd: &BankAuthorizeCmd,
    statepath: &std::path::PathBuf,
) {
    let state = get_state_or_exit(statepath);
    if state.is_token_expired() {
        eprintln!("Token has expired. Maybe refresh?");
        std::process::exit(1);
    }

    let mut auth = banks::Authorize::new(&state.token, &cmd.bank_id);
    let link = auth.start().await.unwrap_or_else(|err| {
        eprintln!("Error starting authorization: {}", err);
        std::process::exit(1);
    });
    println!(
        "Please follow the link below to authenticate with the selected bank."
    );
    println!("  {}", link);

    let requisition = auth.wait_callback().await.unwrap_or_else(|err| {
        eprintln!("Error obtaining bank requisition: {}", err);
        std::process::exit(1);
    });

    let bank_state = banks::BankAuthState::new(&cmd.bank_id, &requisition);
    bank_state.write(&cmd.auth).unwrap_or_else(|err| {
        eprintln!("Error writing bank state: {}", err);
        std::process::exit(1);
    });
    println!("Successfully authorized with bank!");
}

async fn do_bank_account_list(
    statepath: &std::path::PathBuf,
    bankstatepath: &std::path::PathBuf,
) {
    let state = get_state_or_exit(statepath);
    if state.is_token_expired() {
        eprintln!("Token has expired. Maybe refresh?");
        std::process::exit(1);
    }

    let bankstate =
        banks::BankAuthState::parse(bankstatepath).unwrap_or_else(|err| {
            eprintln!(
                "Unable to read bank state file at {}: {}",
                bankstatepath.display(),
                err
            );
            std::process::exit(1);
        });

    let acc = banks::Accounts::new(
        &state.token,
        &bankstate.requisition.requisition_id,
    );

    let acclst = acc.list().await.unwrap_or_else(|err| {
        eprintln!("Unable to list accounts: {}", err);
        std::process::exit(1);
    });

    for account in &acclst {
        let meta = acc.meta(account).await.unwrap_or_else(|err| {
            eprintln!(
                "Error obtaining metadata for account {}: {}",
                account, err
            );
            std::process::exit(1);
        });

        let created_at = match meta.created_at {
            None => String::from("unknown"),
            Some(val) => val.to_string(),
        };
        let accessed_at = match meta.accessed_at {
            None => String::from("unknown"),
            Some(val) => val.to_string(),
        };

        println!("");
        println!("   account id: {}", meta.id);
        println!("         iban: {}", meta.iban);
        println!("     currency: {}", meta.currency);
        println!("      bank id: {}", meta.institution_id);
        if let Some(name) = meta.name {
            println!(" account name: {}", name);
        }
        if let Some(name) = meta.owner_name {
            println!("        owner: {}", name);
        }
        if let Some(product) = meta.product {
            println!("      product: {}", product);
        }
        if let Some(account_type) = meta.account_type {
            println!(" account type: {}", account_type);
        }
        println!("      created: {}", created_at);
        println!("last accessed: {}", accessed_at);
        println!("")
    }
}

async fn do_bank_account_transactions(
    cmd: &BankAccountTransactionsCmd,
    statepath: &std::path::PathBuf,
    bankpath: &std::path::PathBuf,
) {
    let state = get_state_or_exit(statepath);
    if state.is_token_expired() {
        eprintln!("Token has expired. Maybe refresh?");
        std::process::exit(1);
    }

    let bankstate =
        banks::BankAuthState::parse(bankpath).unwrap_or_else(|err| {
            eprintln!(
                "Unable to read bank state file at {}: {}",
                bankpath.display(),
                err
            );
            std::process::exit(1);
        });

    let acc = banks::Accounts::new(
        &state.token,
        &bankstate.requisition.requisition_id,
    );

    let meta_vec = acc.meta_all().await.unwrap_or_else(|err| {
        eprintln!("Error obtaining accounts metadata: {}", err);
        std::process::exit(1);
    });
    let meta = &meta_vec
        .iter()
        .filter(|entry| entry.iban == cmd.iban)
        .take(1)
        .next();

    let selected = match meta {
        None => {
            eprintln!("Could not find account with IBAN {}", cmd.iban);
            std::process::exit(1);
        }
        Some(res) => res,
    };

    let txns = acc.transactions(&selected.id).await.unwrap_or_else(|err| {
        eprintln!("Error obtaining transactions: {}", err);
        std::process::exit(1);
    });

    for tx in &txns.booked {
        let info = match &tx.remittance_information_unstructured {
            None => String::from("<none>"),
            Some(val) => val.clone(),
        };
        println!(
            "{}  {}  {}",
            tx.value_date, tx.transaction_amount.amount, info
        )
    }
}

async fn do_bank_account_balance(
    cmd: &BankAccountBalanceCmd,
    statepath: &std::path::PathBuf,
    bankpath: &std::path::PathBuf,
) {
    let state = get_state_or_exit(statepath);
    if state.is_token_expired() {
        eprintln!("Token has expired. Maybe refresh?");
        std::process::exit(1);
    }

    let bankstate =
        banks::BankAuthState::parse(bankpath).unwrap_or_else(|err| {
            eprintln!(
                "Unable to read bank state file at {}: {}",
                bankpath.display(),
                err
            );
            std::process::exit(1);
        });

    let accnt = banks::Accounts::new(
        &state.token,
        &bankstate.requisition.requisition_id,
    );

    let meta_vec = accnt.meta_all().await.unwrap_or_else(|err| {
        eprintln!("Error obtaining accounts metadata: {}", err);
        std::process::exit(1);
    });
    let meta = &meta_vec
        .iter()
        .filter(|entry| entry.iban == cmd.iban)
        .take(1)
        .next();

    let selected = match meta {
        None => {
            eprintln!("Could not find account with IBAN {}", cmd.iban);
            std::process::exit(1);
        }
        Some(res) => res,
    };

    accnt.balance(&selected.id).await;
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // if let Some(config) = cli.config.as_ref() {
    //     show_config(config);
    // }

    match &cli.command {
        Commands::Authorize(cmd) => {
            do_authorize(cmd).await;
        }
        Commands::Refresh(cmd) => {
            do_refresh(cmd).await;
        }
        Commands::Bank(cmd) => match &cmd.command {
            BankCmds::List(bankcmd) => {
                do_bank_list(bankcmd, &cmd.state).await;
            }
            BankCmds::Authorize(bankcmd) => {
                do_bank_authorization(bankcmd, &cmd.state).await;
            }
            BankCmds::Account(accntcmd) => match &accntcmd.command {
                BankAccountCmds::List(_) => {
                    do_bank_account_list(&cmd.state, &accntcmd.auth).await;
                }
                BankAccountCmds::Transactions(txcmd) => {
                    do_bank_account_transactions(
                        &txcmd,
                        &cmd.state,
                        &accntcmd.auth,
                    )
                    .await;
                }
                BankAccountCmds::Balance(balancecmd) => {
                    do_bank_account_balance(
                        &balancecmd,
                        &cmd.state,
                        &accntcmd.auth,
                    )
                    .await;
                }
            },
        },
    }
}
