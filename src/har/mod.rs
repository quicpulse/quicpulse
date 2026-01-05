//! HAR (HTTP Archive) replay support

pub mod types;
pub mod parser;
pub mod runner;

pub use parser::{load_har, filter_entries, filter_by_indices};
pub use runner::{
    HarRunner, HarReplayOptions,
    format_replay_results, format_har_list, select_requests_interactive, parse_delay
};

use crate::cli::Args;
use crate::errors::QuicpulseError;
use crate::status::ExitStatus;

pub async fn run_har_replay(
    args: &Args,
    har_path: &std::path::Path,
) -> Result<ExitStatus, QuicpulseError> {
    let mut har = load_har(har_path)?;

    if har.log.entries.is_empty() {
        eprintln!("HAR file contains no entries");
        return Ok(ExitStatus::Error);
    }

    if let Some(ref pattern) = args.har_filter {
        filter_entries(&mut har, pattern)?;
        if har.log.entries.is_empty() {
            eprintln!("No entries match filter pattern: {}", pattern);
            return Ok(ExitStatus::Error);
        }
    }

    if !args.har_indices.is_empty() {
        filter_by_indices(&mut har, &args.har_indices);
        if har.log.entries.is_empty() {
            eprintln!("No valid indices specified");
            return Ok(ExitStatus::Error);
        }
    }

    if args.har_list {
        print!("{}", format_har_list(&har));
        return Ok(ExitStatus::Success);
    }

    if args.har_interactive {
        let indices = select_requests_interactive(&har)?;
        if indices.is_empty() {
            eprintln!("No requests selected");
            return Ok(ExitStatus::Success);
        }
        filter_by_indices(&mut har, &indices);
    }

    let delay = if let Some(ref delay_str) = args.har_delay {
        Some(parse_delay(delay_str)?)
    } else {
        None
    };

    let options = HarReplayOptions {
        delay,
        timeout: args.timeout.map(|s| std::time::Duration::from_secs_f64(s)),
        follow_redirects: args.follow,
        verbose: args.verbose > 0,
        dry_run: args.dry_run,
    };

    eprintln!("HAR Replay: {}", har_path.display());
    eprintln!("  Entries to replay: {}", har.log.entries.len());
    if let Some(delay) = options.delay {
        eprintln!("  Delay between requests: {:?}", delay);
    }
    if options.dry_run {
        eprintln!("  Mode: DRY RUN (no requests will be sent)");
    }
    eprintln!();

    let runner = HarRunner::new(options)?;
    let results = runner.replay_all(&har).await;

    print!("{}", format_replay_results(&results));

    let failures = results.iter().filter(|r| r.error.is_some()).count();
    if failures > 0 {
        Ok(ExitStatus::Error)
    } else {
        Ok(ExitStatus::Success)
    }
}
