// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Nereid CLI entrypoint.
//!
//! By default this runs the interactive TUI and serves MCP over streamable HTTP at
//! `http://127.0.0.1:<port>/mcp`.
//!
//! Use `--mcp` to run the MCP server over stdio instead (intended for tool integrations).

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

fn print_usage(program: &str) {
    eprintln!(
        "Usage:\n  {program} [<session-dir>] [--durable-writes] [--mcp-http-port <port>]\n  {program} [--session <dir>] [--durable-writes] [--mcp-http-port <port>]\n  {program} --demo [--mcp-http-port <port>]\n  {program} [<session-dir>] [--durable-writes] --mcp\n  {program} [--session <dir>] [--durable-writes] --mcp\n  {program} --demo --mcp\n\nTUI mode (default) serves MCP over streamable HTTP at `http://127.0.0.1:<port>/mcp`.\n--mcp-http-port selects the port (0 = ephemeral; default {DEFAULT_MCP_HTTP_PORT}).\n\nIf session-dir/--session is omitted, the current working directory is used.\n--demo uses a built-in demo session and cannot be combined with session-dir/--session.\n\n--durable-writes opts into slower, best-effort durable persistence (fsync/sync where supported)."
    );
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct CliOptions {
    mcp: bool,
    demo: bool,
    session_dir: Option<String>,
    mcp_http_port: Option<u16>,
    durable_writes: bool,
}

fn parse_options(mut args: impl Iterator<Item = String>) -> Result<CliOptions, ()> {
    let mut options = CliOptions::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mcp" => {
                if options.mcp {
                    return Err(());
                }
                options.mcp = true;
            }
            "--demo" => {
                if options.demo {
                    return Err(());
                }
                options.demo = true;
            }
            "--session" => {
                if options.session_dir.is_some() {
                    return Err(());
                }
                let dir = args.next().ok_or(())?;
                options.session_dir = Some(dir);
            }
            "--mcp-http-port" => {
                if options.mcp_http_port.is_some() {
                    return Err(());
                }
                let raw = args.next().ok_or(())?;
                let port: u16 = raw.parse().map_err(|_| ())?;
                options.mcp_http_port = Some(port);
            }
            "--durable-writes" => {
                if options.durable_writes {
                    return Err(());
                }
                options.durable_writes = true;
            }
            _ if arg.starts_with('-') => return Err(()),
            _ => {
                if options.session_dir.is_some() {
                    return Err(());
                }
                options.session_dir = Some(arg);
            }
        }
    }

    if options.demo && options.session_dir.is_some() {
        return Err(());
    }

    if options.mcp && options.mcp_http_port.is_some() {
        return Err(());
    }

    Ok(options)
}

fn main() {
    let result = (|| -> Result<(), Box<dyn Error>> {
        let mut args = std::env::args();
        let program = args.next().unwrap_or_else(|| "nereid".to_owned());

        let options = match parse_options(args) {
            Ok(options) => options,
            Err(()) => {
                print_usage(&program);
                std::process::exit(2);
            }
        };

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

            let config = StreamableHttpServerConfig {
                stateful_mode: true,
                ..StreamableHttpServerConfig::default()
            };
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
            tui_result.map_err(|err| {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, err)) as Box<dyn Error>
            })?;
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
    use super::{parse_options, CliOptions};

    #[test]
    fn parses_empty_args() {
        let options = parse_options(std::iter::empty()).expect("parse options");
        assert_eq!(options, CliOptions::default());
    }

    #[test]
    fn parses_demo_flag() {
        let options = parse_options(["--demo".to_owned()].into_iter()).expect("parse options");
        assert!(options.demo);
        assert!(!options.mcp);
        assert!(options.session_dir.is_none());
        assert_eq!(options.mcp_http_port, None);
    }

    #[test]
    fn parses_mcp_flag() {
        let options = parse_options(["--mcp".to_owned()].into_iter()).expect("parse options");
        assert!(options.mcp);
        assert!(!options.demo);
        assert!(options.session_dir.is_none());
        assert_eq!(options.mcp_http_port, None);
    }

    #[test]
    fn parses_session_dir() {
        let options = parse_options(["--session".to_owned(), "some/dir".to_owned()].into_iter())
            .expect("parse options");
        assert_eq!(options.session_dir.as_deref(), Some("some/dir"));
        assert!(!options.mcp);
        assert!(!options.demo);
        assert_eq!(options.mcp_http_port, None);
    }

    #[test]
    fn parses_mcp_http_port() {
        let options = parse_options(["--mcp-http-port".to_owned(), "1234".to_owned()].into_iter())
            .expect("parse options");
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
        let options = parse_options(["--demo".to_owned(), "--mcp".to_owned()].into_iter())
            .expect("parse options");
        assert!(options.demo);
        assert!(options.mcp);

        let options = parse_options(["--mcp".to_owned(), "--demo".to_owned()].into_iter())
            .expect("parse options");
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
        let options = parse_options(["some/dir".to_owned()].into_iter()).expect("parse options");
        assert_eq!(options.session_dir.as_deref(), Some("some/dir"));
        assert!(!options.mcp);
        assert!(!options.demo);
    }

    #[test]
    fn parses_positional_session_dir_with_mcp() {
        let options = parse_options(["some/dir".to_owned(), "--mcp".to_owned()].into_iter())
            .expect("parse options");
        assert_eq!(options.session_dir.as_deref(), Some("some/dir"));
        assert!(options.mcp);
        assert!(!options.demo);
    }

    #[test]
    fn rejects_unknown_args() {
        parse_options(["--nope".to_owned()].into_iter()).unwrap_err();
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
        parse_options(["--session".to_owned()].into_iter()).unwrap_err();
    }
}
