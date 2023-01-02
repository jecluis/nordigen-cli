// nordigen-cli: A simple Nordigen client
// Copyright (C) 2022  Joao Eduardo Luis <joao@abysmo.io>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
use clap::Parser;
use std::io::ErrorKind;

pub mod cli;

use cli::{
    AuthorizeCmd, BankAccountBalanceCmd, BankAuthorizeCmd, BankCmds,
    BankListCmd, Cli, Commands, RefreshCmd,
};
use cli::{BankAccountCmds, BankAccountTransactionsCmd};
use nordigen::banks::BankAuthState;
use nordigen::config::NordigenConfig;
use nordigen::state::NordigenState;
use nordigen::{authorize, banks};
use prettytable::{row, Attr, Cell, Row, Table};

fn read_file(path: &std::path::PathBuf) -> Result<String, String> {
    if !path.exists() {
        return Err(format!("file at {} does not exist!", path.display()));
    }

    let contents = match std::fs::read_to_string(path) {
        Err(error) => {
            return Err(format!(
                "Error reading file at {}: {}",
                path.display(),
                error
            ));
        }
        Ok(value) => value,
    };
    Ok(contents)
}

fn parse_state(path: &std::path::PathBuf) -> Result<NordigenState, String> {
    let contents = match read_file(path) {
        Err(err) => {
            return Err(format!("Error reading state file: {}", err));
        }
        Ok(val) => val,
    };
    let state: NordigenState = match serde_json::from_str(&contents) {
        Err(error) => {
            return Err(format!(
                "Unable to parse state file at {}: {}",
                path.display(),
                error
            ))
        }
        Ok(value) => value,
    };

    Ok(state)
}

fn write_state(
    path: &std::path::PathBuf,
    token: String,
    refresh: String,
    token_ttl: u32,
    refresh_ttl: u32,
) -> Result<NordigenState, String> {
    let state: NordigenState =
        NordigenState::new(token, token_ttl, refresh, refresh_ttl);

    let buffer = match std::fs::File::create(path) {
        Err(err) => {
            return Err(format!(
                "Unable to open state file for writing: {}",
                err
            ));
        }
        Ok(res) => res,
    };

    match serde_json::to_writer_pretty(buffer, &state) {
        Err(err) => {
            return Err(format!("Unable to write state to disk: {}", err));
        }
        Ok(_) => {}
    };

    Ok(state)
}

fn parse_config(path: &std::path::PathBuf) -> Result<NordigenConfig, String> {
    let contents = match read_file(path) {
        Err(err) => {
            return Err(format!("Error reading config file: {}", err));
        }
        Ok(val) => val,
    };
    let config: NordigenConfig = match toml::from_str(&contents) {
        Ok(cfg) => cfg,
        Err(error) => {
            return Err(format!(
                "Unable to parse config file at path {}: {}",
                path.display(),
                error
            ));
        }
    };

    Ok(config)
}

fn parse_bank(path: &std::path::PathBuf) -> Result<BankAuthState, String> {
    let contents = match read_file(path) {
        Err(err) => {
            return Err(format!("Error reading bank file: {}", err));
        }
        Ok(val) => val,
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

fn write_bank<'a>(
    auth: &'a BankAuthState,
    path: &std::path::PathBuf,
) -> Result<&'a BankAuthState, String> {
    let buffer = match std::fs::File::create(path) {
        Err(err) => {
            return Err(format!(
                "Unable to open bank state file for writing: {}",
                err
            ));
        }
        Ok(res) => res,
    };

    match serde_json::to_writer_pretty(buffer, auth) {
        Err(err) => {
            return Err(format!("Unable to write bank state to disk: {}", err));
        }
        Ok(_) => {}
    };
    Ok(auth)
}

fn get_state(path: &std::path::PathBuf) -> Result<NordigenState, ErrorKind> {
    if !path.exists() {
        return Err(ErrorKind::NotFound);
    }

    match parse_state(&path) {
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
        let state = parse_state(&cmd.state).unwrap_or_else(|err| {
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

    let config = parse_config(&cmd.config).unwrap_or_else(|err| {
        println!("Error parsing config: {err}");
        std::process::exit(1);
    });
    println!("config: {}", config);
    let authorization =
        authorize::authorize(&config).await.unwrap_or_else(|err| {
            println!("Error obtaining authorization: {err}");
            std::process::exit(1);
        });

    let state = write_state(
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

    let new_state = write_state(
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
    write_bank(&bank_state, &cmd.auth).unwrap_or_else(|err| {
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

    let bankstate = parse_bank(bankstatepath).unwrap_or_else(|err| {
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

    let bankstate = parse_bank(bankpath).unwrap_or_else(|err| {
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

    let bankstate = parse_bank(bankpath).unwrap_or_else(|err| {
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
