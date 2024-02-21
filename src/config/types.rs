/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   data_type.rs                                       :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: nguiard <nguiard@student.42.fr>            +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2024/02/21 13:39:16 by nguiard           #+#    #+#             */
/*   Updated: 2024/02/21 16:25:11 by nguiard          ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

use std::time::Duration;

use crate::config::signal::Signal;
use serde::Deserialize;
use serde_with::{serde_as, DurationSeconds};

#[derive(Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RestartPolicy {
    #[default]
    Never,
    Always,
    UnexpectedExit,
}

#[derive(Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StartPolicy {
    #[default]
    Auto,
    Manual,
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct Program {
    pub command: String,

    #[serde(default)]
    pub start_policy: StartPolicy,

    #[serde(default = "default_processes")]
    pub processes: u8,

    #[serde(default)]
    #[serde_as(as = "DurationSeconds<u64>")]
    pub min_runtime: Duration,

    #[serde(default)]
    pub valid_exit_codes: Vec<u8>,

    #[serde(default)]
    pub restart_policy: RestartPolicy,

    pub max_restarts: u32,

    #[serde(default)]
    pub valid_signal: Signal,

    #[serde(default)]
    #[serde_as(as = "DurationSeconds<u64>")]
    pub graceful_timeout: Duration,

    pub stdin: Option<String>,

    pub stdout: Option<String>,

    pub env: Option<Vec<String>>,

    pub cwd: Option<String>,

    pub umask: Option<String>,
}

fn default_processes() -> u8 {
    1
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub user: Option<String>,
    pub program: Vec<Program>,
}