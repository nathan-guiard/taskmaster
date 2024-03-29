/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   main.rs                                            :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: nguiard <nguiard@student.42.fr>            +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2024/02/21 10:09:10 by nguiard           #+#    #+#             */
/*   Updated: 2024/02/27 14:00:31 by nguiard          ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

mod config;
mod logger;
mod program;
mod tui;

use config::Config;
use libc::{c_void, sighandler_t, SIGHUP};
use program::StartPolicy;
use std::{
    env::args,
    error::Error,
    mem,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tracing::{error, info, warn};
use tui::{Command, Tui};

static mut RELOAD: AtomicBool = AtomicBool::new(false);
fn sighup_handler() {
    info!("Received SIGHUP");
    unsafe { RELOAD.store(true, Ordering::Relaxed) }
}

fn main() -> Result<(), Box<dyn Error>> {
    let tracing_filter_handle =
        logger::init_logger("taskmaster.log").map_err(|e| format!("starting tracing: {e}"))?;
    let config_path = args().nth(1).unwrap_or("config/default.toml".to_string());
    let mut config = match Config::load(&config_path) {
        Ok(v) => v,
        Err(e) => {
            error!("loading the configuration file {config_path:?}: {e}",);
            return Err(format!("loading the configuration file {config_path}: {e}").into());
        }
    };
    config.tracing_filter_handle = Some(tracing_filter_handle);
    config.reload_tracing_level()?;
    unsafe {
        libc::signal(SIGHUP, sighup_handler as *mut c_void as sighandler_t);
    }
    let mut tui = Tui::new()?;
    for program in &mut config.program {
        if let StartPolicy::Auto = program.start_policy {
            if let Err(e) = program.start() {
                error!(error = e, name = program.name, "starting program");
            }
        }
    }

    let mut pending_quit = false;
    loop {
        if pending_quit && config.program.iter().all(|p| p.all_stopped()) {
            info!("All programs have stopped. Quitting");
            break;
        }
        if unsafe { RELOAD.load(Ordering::Relaxed) } {
            match Config::load(&config_path) {
                Ok(new_config) => config.update(new_config)?,
                Err(e) => {
                    error!(error = e, "reloading the configuration file");
                }
            }
            unsafe { RELOAD.store(false, Ordering::Relaxed) };
        }
        tui.draw(&config.program)?;
        for program in &mut config.program {
            program.tick()?;
        }
        for name in mem::take(&mut config.program_deletions).into_iter() {
            if let Some(position) = config.program.iter().position(|p| p.name == name) {
                if config.program[position].all_stopped() {
                    info!(name, "Removing program");
                    config.program.remove(position);
                } else {
                    config.program_deletions.push(name);
                }
            }
        }
        match tui.tick(Duration::from_millis(10))? {
            Some(Command::Quit) => {
                if pending_quit {
                    warn!("Force quitting");
                    break;
                }
                info!("Gracefully shutting down programs");
                pending_quit = true;
                for program in &mut config.program {
                    program.stop();
                }
            }
            Some(Command::LogLevel(level)) => {
                info!(%level, "Changing log level");
                config.loglevel = level;
                config.reload_tracing_level()?;
            }
            Some(Command::Reload(mut path)) => {
                if path.is_empty() {
                    path = config_path.clone()
                }
                match Config::load(&path) {
                    Ok(new_config) => config.update(new_config)?,
                    Err(e) => {
                        error!(path, error = e, "reloading the configuration file")
                    }
                }
            }
            Some(Command::Start(name)) => {
                if name.is_empty() {
                    info!("Starting all programs");
                    for program in &mut config.program {
                        if let Err(e) = program.start() {
                            error!(error = e, name = program.name, "Starting program");
                        }
                    }
                } else if let Some(p) = config.program.iter_mut().find(|p| p.name == name) {
                    info!(name, "Starting");
                    if let Err(e) = p.start() {
                        error!(error = e, "Starting");
                    }
                } else {
                    error!(name, "Program not found");
                }
            }
            Some(Command::Stop(name)) => {
                if name.is_empty() {
                    info!("Stopping all programs");
                    for program in &mut config.program {
                        program.stop();
                    }
                } else if let Some(p) = config.program.iter_mut().find(|p| p.name == name) {
                    info!(name, "Stopping");
                    p.stop()
                } else {
                    error!(name, "Program not found");
                }
            }
            Some(Command::Restart(name)) => {
                if name.is_empty() {
                    info!("Restarting all programs");
                    for program in &mut config.program {
                        program.restart();
                    }
                } else if let Some(p) = config.program.iter_mut().find(|p| p.name == name) {
                    info!(name, "Restarting");
                    p.restart()
                } else {
                    error!(name, "Program not found");
                }
            }
            None => (),
        }
    }
    // kill in case there is still a program running
    for program in &mut config.program {
        program.kill();
    }
    Ok(())
}
