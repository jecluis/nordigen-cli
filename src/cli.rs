// nordigen-cli: A simple Nordigen client
// Copyright (C) 2022  Joao Eduardo Luis <joao@abysmo.io>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Command to perform
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Authorize application
    Authorize(AuthorizeCmd),
    /// Refresh authorization
    Refresh(RefreshCmd),
    /// Bank related commands
    Bank(BankCmd),
}

#[derive(Args)]
pub struct AuthorizeCmd {
    /// Config file
    #[arg(short, long)]
    pub config: std::path::PathBuf,

    /// State file
    #[arg(short, long)]
    pub state: std::path::PathBuf,
}

#[derive(Args)]
pub struct RefreshCmd {
    /// State file
    #[arg(short, long)]
    pub state: std::path::PathBuf,
}

#[derive(Args)]
#[command()]
pub struct BankCmd {
    /// State file
    #[arg(short, long, required = true, value_name = "FILE")]
    pub state: std::path::PathBuf,

    #[command(subcommand)]
    pub command: BankCmds,
}

#[derive(Subcommand)]
pub enum BankCmds {
    /// List Banks
    List(BankListCmd),
    /// Authorize a Bank
    Authorize(BankAuthorizeCmd),
    /// List Accounts
    Account(BankAccountCmd),
}

#[derive(Args)]
pub struct BankListCmd {
    /// Country to list
    #[arg(short, long, value_name = "CODE")]
    pub country: Option<String>,
}

#[derive(Args)]
pub struct BankAuthorizeCmd {
    /// Bank ID
    pub bank_id: String,

    /// Bank Authorization file
    #[arg(short, long, required = true, value_name = "FILE")]
    pub auth: std::path::PathBuf,
}

#[derive(Args)]
#[command()]
pub struct BankAccountCmd {
    /// Bank Auth State file
    #[arg(short, long, required = true, value_name = "FILE")]
    pub auth: std::path::PathBuf,

    #[command(subcommand)]
    pub command: BankAccountCmds,
}

#[derive(Subcommand)]
pub enum BankAccountCmds {
    List(BankAccountListCmd),
    Transactions(BankAccountTransactionsCmd),
    Balance(BankAccountBalanceCmd),
}

#[derive(Args)]
pub struct BankAccountListCmd {}

#[derive(Args)]
pub struct BankAccountTransactionsCmd {
    /// Account IBAN
    #[arg(short, long, required = true, value_name = "IBAN")]
    pub iban: String,
}

#[derive(Args)]
pub struct BankAccountBalanceCmd {
    /// Account IBAN
    #[arg(short, long, required = true, value_name = "IBAN")]
    pub iban: String,
}
