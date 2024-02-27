/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   child.rs                                           :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: nguiard <nguiard@student.42.fr>            +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2024/02/23 17:47:41 by nguiard           #+#    #+#             */
/*   Updated: 2024/02/27 10:02:11 by nguiard          ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

use super::{Program, RestartPolicy};
use std::{
    error::Error,
    fmt,
    process::{self, ExitStatus},
    time::{Duration, Instant},
};
use tracing::{debug, error, instrument, trace, warn};

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    /// The process is not running
    Stopped(Instant),
    Finished(Instant, ExitStatus),
    /// being gracefully terminated
    Terminating(Instant),
    /// The process is currently starting, but before min_runtime
    Starting(Instant),
    /// after min_runtime
    Running(Instant),
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Stopped(_) => write!(f, "Stopped"),
            Status::Starting(_) => write!(f, "Starting"),
            Status::Terminating(_) => write!(f, "Terminating"),
            Status::Running(_) => write!(f, "Running"),
            Status::Finished(_, code) => write!(f, "Finished ({code})"),
        }
    }
}

#[derive(Debug)]
pub struct Child {
    pub process: process::Child,
    pub status: Status,
    pub restarts: usize,
}

impl Child {
    pub fn new(child: process::Child) -> Self {
        Child {
            process: child,
            status: Status::Starting(Instant::now()),
            restarts: 0,
        }
    }

    pub fn tick(&mut self, program: &mut Program) -> Result<(), Box<dyn Error>> {
        match self.process.try_wait() {
            Ok(Some(status)) if !matches!(self.status, Status::Finished(_, _)) => {
                debug!(
                    pid = self.process.id(),
                    name = program.name,
                    "exit code" = status.code(),
                    "child process finished"
                );
                self.status = Status::Finished(Instant::now(), status);
            }
            Err(e) => {
                warn!(
                    "couldn't get the status of the child process, weird: {:?}",
                    e
                );
            }
            _ => (),
        };
        match self.status {
            Status::Finished(since, code)
                if program.restart_policy == RestartPolicy::UnexpectedExit
                    && since.elapsed() > Duration::from_secs(1) =>
            {
                if !program
                    .valid_exit_codes
                    .contains(&code.code().unwrap_or_default())
                    && ((self.restarts as isize) < program.max_restarts
                        || program.max_restarts == -1)
                {
                    debug!(
                        name = program.name,
                        exit_code = code.code(),
                        "restarting a finished child"
                    );
                    self.restarts += 1;
                    self.process = program.create_child()?.process;
                }
            }
            Status::Finished(since, code)
                if program.restart_policy == RestartPolicy::Always
                    && since.elapsed() > Duration::from_secs(1) =>
            {
                if (self.restarts as isize) < program.max_restarts || program.max_restarts == -1 {
                    debug!(
                        name = program.name,
                        exit_code = code.code(),
                        "restarting a finished child"
                    );
                    self.restarts += 1;
                    self.process = program.create_child()?.process;
                }
            }
            Status::Terminating(since) if program.graceful_timeout < since.elapsed() => {
                warn!(
                    pid = self.process.id(),
                    name = program.name,
                    "graceful shutdown timeout, killing the child"
                );
                self.kill()
            }
            Status::Starting(since) if program.min_runtime < since.elapsed() => {
                trace!(
                    pid = self.process.id(),
                    name = program.name,
                    "child is now considered as running"
                );
                self.status = Status::Running(Instant::now());
            }
            _ => (),
        };
        Ok(())
    }
    /// Kill the child. for graceful shutdown, check stop().
    #[instrument(skip_all)]
    pub fn kill(&mut self) {
        if let Status::Running(_) | Status::Starting(_) = self.status {
            if let Err(e) = self.process.kill() {
                error!(pid = self.process.id(), error = ?e, "couldn't kill the child");
            }
            self.status = Status::Stopped(Instant::now());
        }
    }
    /// gracefully stop the child
    #[instrument(skip_all)]
    pub fn stop(&mut self, signal: i32) {
        if let Status::Running(_) | Status::Starting(_) = self.status {
            if unsafe { libc::kill(self.process.id() as i32, signal) } != 0 {
                error!(pid = self.process.id(), "couldn't send signal to the child");
            }
            self.status = Status::Terminating(Instant::now());
        }
    }

    pub fn last_update(&self) -> Instant {
        match self.status {
            Status::Finished(t, _)
            | Status::Running(t)
            | Status::Stopped(t)
            | Status::Terminating(t)
            | Status::Starting(t) => t,
        }
    }
}
