use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use reqwest::Url;
use serde::Serialize;
use std::path::PathBuf;

mod deploy;
mod onepanel;
mod static_site;

use deploy::{ComposeUpdateOpts, ComposeUpdateResult};
use onepanel::OnePanelConfig;

#[derive(Parser, Debug)]
#[command(name = "1panel-cli", about = "Standalone 1Panel deployment CLI")]
struct Cli {
    #[arg(long, global = true, default_value_t = false)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Deploy static files to a website.
    Deploy {
        #[command(flatten)]
        auth: ServerAuthArgs,
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        group_id: Option<u64>,
        #[arg(long)]
        alias: Option<String>,
        #[arg(long, default_value_t = false)]
        create_if_missing: bool,
        #[arg(short = 'y', long, default_value_t = false)]
        yes: bool,
        #[arg(long, default_value_t = false)]
        non_interactive: bool,
    },

    /// List websites available in 1Panel.
    ListWebsites {
        #[command(flatten)]
        auth: ServerAuthArgs,
    },

    /// Check 1Panel connectivity and auth.
    ServerTest(ServerAuthArgs),

    /// Export local docker image tarball.
    ImageExport {
        #[arg(long)]
        image_tag: String,
        #[arg(long)]
        output: PathBuf,
    },

    /// Upload local image tar to 1Panel.
    ImageUpload {
        #[command(flatten)]
        auth: ServerAuthArgs,
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value = "/opt/1panel/tmp")]
        remote_dir: String,
    },

    /// Load remote image tar on 1Panel.
    DeployLoad {
        #[command(flatten)]
        auth: ServerAuthArgs,
        #[arg(long)]
        remote_path: String,
    },

    /// Update compose image references on 1Panel.
    DeployComposeUpdate {
        #[command(flatten)]
        auth: ServerAuthArgs,
        #[arg(long)]
        compose_name: String,
        #[arg(long)]
        compose_path: String,
        #[arg(long)]
        to_image: String,
        #[arg(long)]
        service: Option<String>,
        #[arg(long)]
        from_image: Option<String>,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value_t = false)]
        apply: bool,
    },

    /// Full pipeline: export -> upload -> load.
    DeployAll {
        #[command(flatten)]
        auth: ServerAuthArgs,
        #[arg(long)]
        image_tag: String,
        #[arg(long, default_value = "/opt/1panel/tmp")]
        remote_dir: String,
        #[arg(long, default_value_t = false)]
        keep_local_tar: bool,
    },
}

#[derive(clap::Args, Debug, Clone)]
struct ServerAuthArgs {
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    api_key: Option<String>,
    #[arg(long)]
    base_url: Option<String>,
}

#[derive(Serialize)]
struct OkMessage<'a> {
    ok: bool,
    message: &'a str,
}

fn parse_base_url(base_url: &str) -> Result<(String, u16)> {
    let normalized = if base_url.starts_with("http://") || base_url.starts_with("https://") {
        base_url.to_string()
    } else {
        format!("http://{}", base_url)
    };

    let url = Url::parse(&normalized).map_err(|e| anyhow!("invalid --base-url: {}", e))?;
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("invalid --base-url: missing host"))?
        .to_string();
    let port = url.port_or_known_default().unwrap_or(9999);

    Ok((host, port))
}

fn cfg_from(auth: &ServerAuthArgs) -> Result<OnePanelConfig> {
    let env_host = std::env::var("ONEPANEL_HOST").ok();
    let env_port = std::env::var("ONEPANEL_PORT").ok().and_then(|v| v.parse::<u16>().ok());
    let env_api_key = std::env::var("ONEPANEL_API_KEY").ok();
    let env_base_url = std::env::var("ONEPANEL_BASE_URL").ok();

    let base_url = auth.base_url.clone().or(env_base_url);
    let parsed_base = match base_url {
        Some(url) => Some(parse_base_url(&url)?),
        None => None,
    };

    let host = auth
        .host
        .clone()
        .or(env_host)
        .or_else(|| parsed_base.as_ref().map(|(h, _)| h.clone()))
        .ok_or_else(|| anyhow!("host is required (use --host, --base-url, ONEPANEL_HOST, or ONEPANEL_BASE_URL)"))?;

    let port = auth
        .port
        .or(env_port)
        .or_else(|| parsed_base.as_ref().map(|(_, p)| *p))
        .unwrap_or(9999);

    let api_key = auth
        .api_key
        .clone()
        .or(env_api_key)
        .ok_or_else(|| anyhow!("api key is required (use --api-key or ONEPANEL_API_KEY)"))?;

    Ok(OnePanelConfig::new(host, port, api_key))
}

fn print_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).unwrap());
}

fn print_compose_result_human(result: &ComposeUpdateResult) {
    println!("changed={} dry_run={}", result.changed, result.dry_run);
    for c in &result.changes {
        println!("{}: {} -> {}", c.service, c.from, c.to);
    }
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Deploy {
            auth,
            path,
            domain,
            group_id,
            alias,
            create_if_missing,
            yes,
            non_interactive: _,
        } => {
            let cfg = cfg_from(&auth)?;
            let result = static_site::deploy_static(&cfg, &path, domain, create_if_missing || yes, alias.as_deref(), group_id).await?;

            if cli.json {
                print_json(&result);
            } else {
                println!("Deployment completed successfully.");
                println!("Website: {}", result.url);
                println!("Files uploaded: {}/{}", result.uploaded, result.total);
            }
        }
        Commands::ListWebsites { auth } => {
            let cfg = cfg_from(&auth)?;
            let websites = static_site::list_websites(&cfg).await?;
            let payload = serde_json::json!({
                "ok": true,
                "action": "list-websites",
                "count": websites.len(),
                "websites": websites,
            });

            if cli.json {
                print_json(&payload);
            } else if websites.is_empty() {
                println!("No websites found.");
            } else {
                for w in websites {
                    println!("{}", w.domain);
                }
            }
        }
        Commands::ServerTest(auth) => {
            onepanel::test_connection(&cfg_from(&auth)?).await?;
            if cli.json {
                print_json(&OkMessage {
                    ok: true,
                    message: "connection successful",
                });
            } else {
                println!("connection successful");
            }
        }
        Commands::ImageExport { image_tag, output } => {
            let result = deploy::export_image(&image_tag, &output).await?;
            if cli.json {
                print_json(&result);
            } else {
                println!("exported: {}", result.tar_path);
            }
        }
        Commands::ImageUpload {
            auth,
            input,
            remote_dir,
        } => {
            let result = deploy::upload_image_tar(&cfg_from(&auth)?, &input, &remote_dir).await?;
            if cli.json {
                print_json(&result);
            } else {
                println!("uploaded: {}", result.remote_path);
            }
        }
        Commands::DeployLoad { auth, remote_path } => {
            deploy::load_remote_image(&cfg_from(&auth)?, &remote_path).await?;
            if cli.json {
                print_json(&OkMessage {
                    ok: true,
                    message: "image loaded",
                });
            } else {
                println!("image loaded");
            }
        }
        Commands::DeployComposeUpdate {
            auth,
            compose_name,
            compose_path,
            to_image,
            service,
            from_image,
            dry_run,
            apply,
        } => {
            if service.is_none() && from_image.is_none() {
                return Err(anyhow!(
                    "compose update requires --service or --from-image to avoid accidental global replacement"
                ));
            }

            let result = deploy::run_compose_update(
                &cfg_from(&auth)?,
                ComposeUpdateOpts {
                    compose_name,
                    compose_path,
                    service,
                    from_image,
                    to_image,
                    dry_run,
                    apply,
                },
            )
            .await?;

            if cli.json {
                print_json(&result);
            } else {
                print_compose_result_human(&result);
            }
        }
        Commands::DeployAll {
            auth,
            image_tag,
            remote_dir,
            keep_local_tar,
        } => {
            let (export, upload) =
                deploy::deploy_all(&cfg_from(&auth)?, &image_tag, &remote_dir, keep_local_tar).await?;

            if cli.json {
                print_json(&serde_json::json!({
                    "export": export,
                    "upload": upload,
                    "loaded": true
                }));
            } else {
                println!("exported: {}", export.tar_path);
                println!("uploaded: {}", upload.remote_path);
                println!("image loaded");
            }
        }
    }

    Ok(())
}
