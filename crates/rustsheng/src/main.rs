// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev <mkrutovercev@yandex.ru> (RD2W)
// Derived from k5prog (Jacek Lipkowski, SQ5BPF), k5prog-win (OneOfEleven), and K5TOOL (qrp73).

//! `rustsheng` command-line front-end entry point.

mod cli;

use clap::Parser;

use cli::Cli;

fn main() {
    let args = Cli::parse();
    init_logging(args.verbose);
    if let Err(e) = cli::commands::dispatch(args.command) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

/// Maps `-v` occurrences to a log level and initializes `env_logger`.
fn init_logging(verbose: u8) {
    let level = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    env_logger::Builder::new()
        .parse_filters(level)
        .format_timestamp(None)
        .init();
}
