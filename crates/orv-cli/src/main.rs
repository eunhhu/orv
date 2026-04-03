use clap::Parser;

#[derive(Parser)]
#[command(name = "orv", version, about = "Integrated Platform Development DSL")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Display version information
    Version,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Version) => {
            println!("orv {}", orv_core::version());
        }
        None => {
            println!("orv {}", orv_core::version());
        }
    }

    Ok(())
}
