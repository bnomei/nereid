// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! CLI entrypoint: TUI + HTTP MCP, or stdio MCP.
//!
//! Default: load/init a session folder (or demo), start the TUI, and serve MCP on
//! `http://127.0.0.1:<port>/mcp`. `--mcp` runs stdio-only for agent hosts. Also supports
//! `--dump-mcp-tool-schema` for schema snapshot regeneration.

use std::collections::BTreeSet;
use std::error::Error;
use std::sync::Arc;

use axum::Router;
use rmcp::transport::{
    streamable_http_server::session::local::LocalSessionManager, StreamableHttpServerConfig,
    StreamableHttpService,
};
use tokio::sync::Mutex;

const DEFAULT_MCP_HTTP_PORT: u16 = 27435;

fn usage(program: &str) -> String {
    format!(
        "Usage:\n  {program} [<session-dir>] [--durable-writes] [--mcp-http-port <port>]\n  {program} [--session <dir>] [--durable-writes] [--mcp-http-port <port>]\n  {program} --demo [--mcp-http-port <port>]\n  {program} [<session-dir>] [--durable-writes] --mcp\n  {program} [--session <dir>] [--durable-writes] --mcp\n  {program} --demo --mcp\n  {program} --dump-mcp-tool-schema\n\nTUI mode (default) serves MCP over streamable HTTP at `http://127.0.0.1:<port>/mcp`.\n--mcp-http-port selects the port (0 = ephemeral; default {DEFAULT_MCP_HTTP_PORT}).\n\nIf session-dir/--session is omitted, the current working directory is used.\n--demo uses a built-in demo session and cannot be combined with session-dir/--session.\n\n--dump-mcp-tool-schema prints the stable MCP tool schema snapshot and exits.\n--durable-writes opts into slower, best-effort durable persistence (fsync/sync where supported)."
    )
}

fn print_usage(program: &str) {
    eprintln!("{}", usage(program));
}

fn version() -> String {
    format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Run(CliOptions),
    Help,
    Version,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct CliOptions {
    mcp: bool,
    demo: bool,
    session_dir: Option<String>,
    mcp_http_port: Option<u16>,
    durable_writes: bool,
    dump_mcp_tool_schema: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliParseError {
    DuplicateFlag(&'static str),
    MissingValue(&'static str),
    InvalidValue { flag: &'static str, value: String, expected: &'static str },
    UnknownFlag(String),
    UnexpectedArgument(String),
    ConflictingOptions(&'static str),
}

impl std::fmt::Display for CliParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateFlag(flag) => write!(f, "duplicate option `{flag}`"),
            Self::MissingValue(flag) => write!(f, "missing value for `{flag}`"),
            Self::InvalidValue { flag, value, expected } => {
                write!(f, "invalid value `{value}` for `{flag}` (expected {expected})")
            }
            Self::UnknownFlag(flag) => write!(f, "unknown option `{flag}`"),
            Self::UnexpectedArgument(arg) => {
                write!(f, "unexpected argument `{arg}`; only one session directory may be provided")
            }
            Self::ConflictingOptions(message) => f.write_str(message),
        }
    }
}

fn parse_options(mut args: impl Iterator<Item = String>) -> Result<CliCommand, CliParseError> {
    let mut options = CliOptions::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mcp" => {
                if options.mcp {
                    return Err(CliParseError::DuplicateFlag("--mcp"));
                }
                options.mcp = true;
            }
            "--demo" => {
                if options.demo {
                    return Err(CliParseError::DuplicateFlag("--demo"));
                }
                options.demo = true;
            }
            "--session" => {
                if options.session_dir.is_some() {
                    return Err(CliParseError::DuplicateFlag("--session"));
                }
                let dir = args
                    .next()
                    .filter(|value| !value.starts_with('-'))
                    .ok_or(CliParseError::MissingValue("--session"))?;
                options.session_dir = Some(dir);
            }
            "--mcp-http-port" => {
                if options.mcp_http_port.is_some() {
                    return Err(CliParseError::DuplicateFlag("--mcp-http-port"));
                }
                let raw = args
                    .next()
                    .filter(|value| !value.starts_with('-'))
                    .ok_or(CliParseError::MissingValue("--mcp-http-port"))?;
                let port: u16 = raw.parse().map_err(|_| CliParseError::InvalidValue {
                    flag: "--mcp-http-port",
                    value: raw.clone(),
                    expected: "a TCP port from 0 to 65535",
                })?;
                options.mcp_http_port = Some(port);
            }
            "--durable-writes" => {
                if options.durable_writes {
                    return Err(CliParseError::DuplicateFlag("--durable-writes"));
                }
                options.durable_writes = true;
            }
            "--dump-mcp-tool-schema" => {
                if options.dump_mcp_tool_schema {
                    return Err(CliParseError::DuplicateFlag("--dump-mcp-tool-schema"));
                }
                options.dump_mcp_tool_schema = true;
            }
            "--help" | "-h" => return Ok(CliCommand::Help),
            "--version" | "-V" => return Ok(CliCommand::Version),
            _ if arg.starts_with('-') => return Err(CliParseError::UnknownFlag(arg)),
            _ => {
                if options.session_dir.is_some() {
                    return Err(CliParseError::UnexpectedArgument(arg));
                }
                options.session_dir = Some(arg);
            }
        }
    }

    if options.demo && options.session_dir.is_some() {
        return Err(CliParseError::ConflictingOptions(
            "`--demo` cannot be combined with a session directory or `--session`",
        ));
    }

    if options.mcp && options.mcp_http_port.is_some() {
        return Err(CliParseError::ConflictingOptions(
            "`--mcp-http-port` cannot be combined with stdio MCP mode (`--mcp`)",
        ));
    }

    if options.dump_mcp_tool_schema
        && (options.mcp
            || options.demo
            || options.session_dir.is_some()
            || options.mcp_http_port.is_some()
            || options.durable_writes)
    {
        return Err(CliParseError::ConflictingOptions(
            "`--dump-mcp-tool-schema` cannot be combined with runtime options",
        ));
    }

    Ok(CliCommand::Run(options))
}

fn main() {
    let result = (|| -> Result<(), Box<dyn Error>> {
        let mut args = std::env::args();
        let program = args.next().unwrap_or_else(|| "nereid".to_owned());

        let options = match parse_options(args) {
            Ok(CliCommand::Run(options)) => options,
            Ok(CliCommand::Help) => {
                println!("{}", usage(&program));
                return Ok(());
            }
            Ok(CliCommand::Version) => {
                println!("{}", version());
                return Ok(());
            }
            Err(err) => {
                eprintln!("nereid: {err}");
                print_usage(&program);
                std::process::exit(2);
            }
        };

        if options.dump_mcp_tool_schema {
            print!("{}", nereid::mcp::NereidMcp::tool_schema_snapshot()?);
            return Ok(());
        }

        if options.mcp {
            let mcp = if options.demo {
                let session = nereid::tui::demo_session();
                nereid::mcp::NereidMcp::new(session)
            } else {
                let dir = options.session_dir.unwrap_or_else(|| ".".to_owned());
                let folder = if options.durable_writes {
                    nereid::store::SessionFolder::new(dir)
                        .with_durability(nereid::store::WriteDurability::Durable)
                } else {
                    nereid::store::SessionFolder::new(dir)
                };
                let session = folder.load_or_init_session()?;
                nereid::mcp::NereidMcp::new_persistent(session, folder)
            };

            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

            runtime.block_on(mcp.serve_stdio())?;
            return Ok(());
        }

        let agent_highlights = Arc::new(Mutex::new(BTreeSet::new()));
        let ui_state = Arc::new(Mutex::new(nereid::ui::UiState::default()));
        let mcp_http_port = options.mcp_http_port.unwrap_or(DEFAULT_MCP_HTTP_PORT);

        let (tui_session, tui_session_folder, mcp) = if options.demo {
            // In demo mode we still need a shared persistence channel so TUI and MCP can
            // synchronize multi-selection and other session mutations.
            let now_millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let demo_dir = std::env::temp_dir()
                .join(format!("nereid-demo-session-{}-{now_millis}", std::process::id()));
            let folder = if options.durable_writes {
                nereid::store::SessionFolder::new(demo_dir)
                    .with_durability(nereid::store::WriteDurability::Durable)
            } else {
                nereid::store::SessionFolder::new(demo_dir)
            };
            let session = nereid::tui::demo_session();
            folder.save_session(&session)?;
            let tui_session = session.clone();
            let tui_session_folder = folder.clone();
            let mcp = nereid::mcp::NereidMcp::new_persistent_with_agent_highlights_and_ui_state(
                session,
                folder,
                agent_highlights.clone(),
                Some(ui_state.clone()),
            );
            (tui_session, Some(tui_session_folder), mcp)
        } else {
            let dir = options.session_dir.unwrap_or_else(|| ".".to_owned());
            let folder = if options.durable_writes {
                nereid::store::SessionFolder::new(dir)
                    .with_durability(nereid::store::WriteDurability::Durable)
            } else {
                nereid::store::SessionFolder::new(dir)
            };
            let session = folder.load_or_init_session()?;
            let tui_session = session.clone();
            let tui_session_folder = folder.clone();
            let mcp = nereid::mcp::NereidMcp::new_persistent_with_agent_highlights_and_ui_state(
                session,
                folder,
                agent_highlights.clone(),
                Some(ui_state.clone()),
            );
            (tui_session, Some(tui_session_folder), mcp)
        };

        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", mcp_http_port)).await?;

            let config = StreamableHttpServerConfig::default().with_stateful_mode(true);
            let shutdown_token = config.cancellation_token.clone();
            let server_shutdown = shutdown_token.clone();

            let session_manager = Arc::new(LocalSessionManager::default());
            let mcp_service = {
                let mcp = mcp.clone();
                StreamableHttpService::new(move || Ok(mcp.clone()), session_manager, config)
            };

            let router = Router::new().nest_service("/mcp", mcp_service);
            let server_handle = tokio::spawn(async move {
                let serve = axum::serve(listener, router).with_graceful_shutdown(async move {
                    server_shutdown.cancelled().await;
                });
                if let Err(err) = serve.await {
                    eprintln!("nereid: MCP HTTP server error: {err}");
                }
            });

            let tui_agent_highlights = agent_highlights.clone();
            let tui_ui_state = ui_state.clone();
            let tui_join = tokio::task::spawn_blocking(move || {
                nereid::tui::run_with_session_with_ui_state(
                    tui_session,
                    tui_agent_highlights,
                    Some(tui_ui_state),
                    tui_session_folder,
                )
                .map_err(|err| err.to_string())
            })
            .await;

            shutdown_token.cancel();
            let _ = server_handle.await;

            let tui_result = tui_join.map_err(|err| -> Box<dyn Error> { Box::new(err) })?;
            tui_result.map_err(|err| Box::new(std::io::Error::other(err)) as Box<dyn Error>)?;
            Ok::<(), Box<dyn Error>>(())
        })?;

        Ok(())
    })();

    if let Err(err) = result {
        eprintln!("nereid: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_options, usage, version, CliCommand, CliOptions, CliParseError};

    fn unwrap_run(command: CliCommand) -> CliOptions {
        match command {
            CliCommand::Run(options) => options,
            other => panic!("expected run command, got {other:?}"),
        }
    }

    #[test]
    fn parses_empty_args() {
        let options = unwrap_run(parse_options(std::iter::empty()).expect("parse options"));
        assert_eq!(options, CliOptions::default());
    }

    #[test]
    fn parses_demo_flag() {
        let options =
            unwrap_run(parse_options(["--demo".to_owned()].into_iter()).expect("parse options"));
        assert!(options.demo);
        assert!(!options.mcp);
        assert!(options.session_dir.is_none());
        assert_eq!(options.mcp_http_port, None);
    }

    #[test]
    fn parses_dump_mcp_tool_schema_flag() {
        let options = unwrap_run(
            parse_options(["--dump-mcp-tool-schema".to_owned()].into_iter())
                .expect("parse options"),
        );
        assert!(options.dump_mcp_tool_schema);
        assert!(!options.mcp);
        assert!(!options.demo);
    }

    #[test]
    fn parses_mcp_flag() {
        let options =
            unwrap_run(parse_options(["--mcp".to_owned()].into_iter()).expect("parse options"));
        assert!(options.mcp);
        assert!(!options.demo);
        assert!(options.session_dir.is_none());
        assert_eq!(options.mcp_http_port, None);
    }

    #[test]
    fn parses_session_dir() {
        let options = unwrap_run(
            parse_options(["--session".to_owned(), "some/dir".to_owned()].into_iter())
                .expect("parse options"),
        );
        assert_eq!(options.session_dir.as_deref(), Some("some/dir"));
        assert!(!options.mcp);
        assert!(!options.demo);
        assert_eq!(options.mcp_http_port, None);
    }

    #[test]
    fn parses_mcp_http_port() {
        let options = unwrap_run(
            parse_options(["--mcp-http-port".to_owned(), "1234".to_owned()].into_iter())
                .expect("parse options"),
        );
        assert_eq!(options.mcp_http_port, Some(1234));
        assert!(!options.mcp);
    }

    #[test]
    fn rejects_mcp_http_port_with_stdio_mcp_mode() {
        parse_options(
            ["--mcp".to_owned(), "--mcp-http-port".to_owned(), "0".to_owned()].into_iter(),
        )
        .unwrap_err();
    }

    #[test]
    fn parses_demo_and_mcp_in_any_order() {
        let options = unwrap_run(
            parse_options(["--demo".to_owned(), "--mcp".to_owned()].into_iter())
                .expect("parse options"),
        );
        assert!(options.demo);
        assert!(options.mcp);

        let options = unwrap_run(
            parse_options(["--mcp".to_owned(), "--demo".to_owned()].into_iter())
                .expect("parse options"),
        );
        assert!(options.demo);
        assert!(options.mcp);
    }

    #[test]
    fn rejects_demo_with_session_dir() {
        parse_options(["--demo".to_owned(), "--session".to_owned(), ".".to_owned()].into_iter())
            .unwrap_err();
    }

    #[test]
    fn parses_positional_session_dir() {
        let options =
            unwrap_run(parse_options(["some/dir".to_owned()].into_iter()).expect("parse options"));
        assert_eq!(options.session_dir.as_deref(), Some("some/dir"));
        assert!(!options.mcp);
        assert!(!options.demo);
    }

    #[test]
    fn parses_positional_session_dir_with_mcp() {
        let options = unwrap_run(
            parse_options(["some/dir".to_owned(), "--mcp".to_owned()].into_iter())
                .expect("parse options"),
        );
        assert_eq!(options.session_dir.as_deref(), Some("some/dir"));
        assert!(options.mcp);
        assert!(!options.demo);
    }

    #[test]
    fn rejects_unknown_args() {
        parse_options(["--nope".to_owned()].into_iter()).unwrap_err();
    }

    #[test]
    fn rejects_dump_mcp_tool_schema_with_runtime_options() {
        parse_options(["--dump-mcp-tool-schema".to_owned(), "--mcp".to_owned()].into_iter())
            .unwrap_err();
    }

    #[test]
    fn rejects_duplicate_flags() {
        parse_options(["--demo".to_owned(), "--demo".to_owned()].into_iter()).unwrap_err();

        parse_options(["--mcp".to_owned(), "--mcp".to_owned()].into_iter()).unwrap_err();

        parse_options(
            ["--session".to_owned(), ".".to_owned(), "--session".to_owned(), "other".to_owned()]
                .into_iter(),
        )
        .unwrap_err();
    }

    #[test]
    fn rejects_multiple_positional_session_dirs() {
        parse_options(["one".to_owned(), "two".to_owned()].into_iter()).unwrap_err();
    }

    #[test]
    fn rejects_positional_session_dir_with_session_flag() {
        parse_options(["--session".to_owned(), "one".to_owned(), "two".to_owned()].into_iter())
            .unwrap_err();
    }

    #[test]
    fn rejects_missing_session_value() {
        assert_eq!(
            parse_options(["--session".to_owned()].into_iter()).unwrap_err(),
            CliParseError::MissingValue("--session")
        );
        assert_eq!(
            parse_options(["--session".to_owned(), "--mcp".to_owned()].into_iter()).unwrap_err(),
            CliParseError::MissingValue("--session")
        );
    }

    #[test]
    fn parses_help_and_version_commands() {
        assert_eq!(
            parse_options(["--help".to_owned()].into_iter()).expect("parse help"),
            CliCommand::Help
        );
        assert_eq!(
            parse_options(["--version".to_owned()].into_iter()).expect("parse version"),
            CliCommand::Version
        );
    }

    #[test]
    fn formats_usage_and_version_output() {
        let help = usage("nereid");
        assert!(help.contains("Usage:"));
        assert!(help.contains("--session <dir>"));
        assert!(help.contains("--mcp-http-port <port>"));
        assert!(help.contains("--dump-mcp-tool-schema"));
        assert_eq!(version(), format!("nereid {}", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn reports_actionable_parse_errors() {
        assert_eq!(
            parse_options(["--nope".to_owned()].into_iter()).unwrap_err().to_string(),
            "unknown option `--nope`"
        );
        assert_eq!(
            parse_options(["--mcp-http-port".to_owned(), "abc".to_owned()].into_iter())
                .unwrap_err()
                .to_string(),
            "invalid value `abc` for `--mcp-http-port` (expected a TCP port from 0 to 65535)"
        );
    }
}
