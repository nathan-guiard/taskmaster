/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   terminal_status.rs                                 :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: nguiard <nguiard@student.42.fr>            +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2024/02/26 17:22:15 by nguiard           #+#    #+#             */
/*   Updated: 2024/02/27 12:48:53 by nguiard          ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

use crate::program::{child::Status, Program};

use std::{process::ExitStatus, time::Instant};
use ratatui::{style::Style, widgets::{Cell, Row}};

impl Program {
	/// Terminal status
	fn status_global(&self, status_check: Status) -> Option<Row> {
		let since = self.childs.iter()
								.filter(|&c| c.status.eq(&status_check))
								.max_by_key(|x| x.last_update());
		let running: usize = self.childs.iter()
								.filter(|&c| c.status.eq(&status_check))
								.count();
		if running == 0{
			return None;
		};
		let since_str = match since {
			Some(c) => format!("{:?}", c.last_update().elapsed()),
			None => "Unknown".to_string(),
		};
		let status_str = format!("{running}/{}", self.childs.len());

		Some(Row::new(vec![
			Cell::from(self.name.clone()),
			Cell::from(format!("{}", status_check).to_string())
					.style(Style::new().fg(status_check.color())),
			Cell::from(status_str),
			Cell::from(since_str)
		]))
	}
	
	pub fn status(&self) -> Vec<Row> {
		let instant_dummy = Instant::now();
		let running = self.status_global(Status::Running(instant_dummy));
		let terminating = self.status_global(Status::Terminating(instant_dummy));
		let starting = self.status_global(Status::Starting(instant_dummy));
		let finished = self.status_global(Status::Finished(instant_dummy, ExitStatus::default()));
		let stopped = self.status_global(Status::Stopped(instant_dummy));
		let mut res_lines: Vec<Row> = vec![];

		for line in [running, starting, terminating, stopped, finished]
					.into_iter()
					.flatten() {
			res_lines.push(line);
		}

		if res_lines.is_empty() {
			// Safe because status_running always return a value if force == true
			res_lines.push(Row::new([
				self.name.clone(),
				"Not launched".to_string(),
				"No processes".to_string(),
				"Unknown".to_string()
			]));
		}
		
		res_lines
	}
}