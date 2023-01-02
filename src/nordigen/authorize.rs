// nordigen-cli: A simple Nordigen client
// Copyright (C) 2022  Joao Eduardo Luis <joao@abysmo.io>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//

use std::collections::HashMap;

use serde::Deserialize;

use crate::nordigen::config::NordigenConfig;

#[derive(Deserialize)]
pub struct AuthorizeReply {
    pub access: String,
    pub access_expires: u32,
    pub refresh: String,
    pub refresh_expires: u32,
}

#[derive(Deserialize)]
struct RefreshReply {
    pub access: String,
    pub access_expires: u32,
}

pub async fn authorize(
    config: &NordigenConfig,
) -> Result<AuthorizeReply, String> {
    let mut map: HashMap<&str, &String> = HashMap::new();
    map.insert("secret_id", &config.secret_id);
    map.insert("secret_key", &config.secret_key);

    let client = reqwest::Client::new();
    let res = match client
        .post("https://ob.nordigen.com/api/v2/token/new/")
        .header("accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&map)
        .send()
        .await
    {
        Err(error) => {
            return Err(format!("Unable to obtain token: {}", error));
        }
        Ok(res) => res,
    };

    let value: AuthorizeReply = match res.json::<AuthorizeReply>().await {
        Err(error) => {
            return Err(format!("Unable to obtain response value: {error}"));
        }
        Ok(res) => res,
    };
    println!("authorization:");
    println!("   access token: {}", value.access);
    println!("  refresh token: {}", value.refresh);

    Ok(value)
}

pub async fn refresh(refresh_token: &String) -> Result<(String, u32), String> {
    let mut map: HashMap<&str, &String> = HashMap::new();
    map.insert("refresh", refresh_token);

    let client = reqwest::Client::new();
    let res = match client
        .post("https://ob.nordigen.com/api/v2/token/refresh/")
        .header("accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&map)
        .send()
        .await
    {
        Err(error) => {
            return Err(format!("Unable to refresh token: {}", error));
        }
        Ok(res) => res,
    };

    let value: RefreshReply = match res.json::<RefreshReply>().await {
        Err(error) => {
            return Err(format!("Unable to obtain response value: {}", error));
        }
        Ok(res) => res,
    };

    Ok((value.access, value.access_expires))
}
