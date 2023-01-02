// nordigen-cli: A simple Nordigen client
// Copyright (C) 2022  Joao Eduardo Luis <joao@abysmo.io>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct NordigenConfig {
    pub secret_id: String,
    pub secret_key: String,
}

impl NordigenConfig {
    pub fn parse(path: &std::path::PathBuf) -> Result<Self, String> {
        if !path.exists() {
            return Err(format!(
                "Config file at {} does not exist!",
                path.display()
            ));
        }

        let contents = match fs::read_to_string(path) {
            Err(error) => {
                return Err(format!(
                    "Error reading file at path {}: {}",
                    path.display(),
                    error
                ));
            }
            Ok(cfg) => cfg,
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
}

impl std::fmt::Display for NordigenConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            " secret(id: {}, key: {})",
            self.secret_id, self.secret_key
        )
    }
}
