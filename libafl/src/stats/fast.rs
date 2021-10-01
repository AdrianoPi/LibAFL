//! Stats to disply both cumulative and per-client stats

use alloc::{string::String, vec::Vec};
use core::{time, time::Duration};

#[cfg(feature = "introspection")]
use alloc::string::ToString;

use crate::{
    bolts::current_time,
    stats::{ClientStats, Stats},
};

use std::fs;

/// Tracking stats during fuzzing and display both per-client and cumulative info.
#[derive(Clone, Debug)]
pub struct FastStats
{
    start_time: Duration,
    last_print_time: Duration,
    corpus_size: usize,
    client_stats: Vec<ClientStats>,
}

impl Stats for FastStats
{
    /// the client stats, mutable
    fn client_stats_mut(&mut self) -> &mut Vec<ClientStats> {
        &mut self.client_stats
    }

    /// the client stats
    fn client_stats(&self) -> &[ClientStats] {
        &self.client_stats
    }

    /// Time this fuzzing run stated
    fn start_time(&mut self) -> time::Duration {
        self.start_time
    }

    fn display(&mut self, event_msg: String, sender_id: u32) {
        let cur_time = current_time();
        if cur_time.as_secs() + 2 < self.last_print_time.as_secs() {
            return
        }
        self.last_print_time = cur_time;
        fs::write(format!("fuzz{}.txt", sender_id), format!("{}", self.total_execs()));
    }
}

impl FastStats
{
    /// Creates the stats, using the `current_time` as `start_time`.
    pub fn new() -> Self {
        Self {
            start_time: current_time(),
            last_print_time: current_time(),
            corpus_size: 0,
            client_stats: vec![],
        }
    }

    /// Creates the stats with a given `start_time`.
    pub fn with_time(start_time: time::Duration) -> Self {
        Self {
            start_time,
            last_print_time: start_time,
            corpus_size: 0,
            client_stats: vec![],
        }
    }
}
