mod commands;

use clap::{Parser, Subcommand};
use commands::{CommandContext, Format, IconFitArg};

#[derive(Debug, Parser)]
#[command(
    name = "pevi",
    version,
    about = "Inspect and update Windows PE VERSIONINFO and icons"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init {
        #[arg(short, long, default_value = "pevi.toml")]
        output: std::path::PathBuf,
        #[arg(long)]
        force: bool,
    },
    Inspect {
        #[arg(long)]
        input: std::path::PathBuf,
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
    Plan {
        #[arg(long)]
        config: std::path::PathBuf,
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
        #[arg(long)]
        in_place: bool,
        #[arg(long)]
        confirm_in_place: bool,
        #[arg(long)]
        allow_signed_input: bool,
        #[arg(long)]
        acknowledge_signature_invalidation: bool,
    },
    Apply {
        #[arg(long)]
        config: std::path::PathBuf,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
        #[arg(long)]
        in_place: bool,
        #[arg(long)]
        confirm_in_place: bool,
        #[arg(long)]
        allow_signed_input: bool,
        #[arg(long)]
        acknowledge_signature_invalidation: bool,
    },
    Verify {
        #[arg(long)]
        input: std::path::PathBuf,
        #[arg(long)]
        config: Option<std::path::PathBuf>,
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
    ConvertIcon {
        #[arg(long)]
        input: std::path::PathBuf,
        #[arg(long)]
        output: std::path::PathBuf,
        #[arg(long, value_enum, default_value_t = IconFitArg::Contain)]
        fit: IconFitArg,
        #[arg(long)]
        allow_crop: bool,
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
}

fn main() {
    let cli = Cli::parse();
    let context = CommandContext::new();
    let (command_name, format, result) = match cli.command {
        Commands::Init { output, force } => ("init", Format::Human, context.init(&output, force)),
        Commands::Inspect { input, format } => ("inspect", format, context.inspect(&input, format)),
        Commands::Plan {
            config,
            format,
            in_place,
            confirm_in_place,
            allow_signed_input,
            acknowledge_signature_invalidation,
        } => (
            "plan",
            format,
            context.plan(
                &config,
                format,
                commands::AuthorizationFlags {
                    in_place,
                    confirm_in_place,
                    allow_signed_input,
                    acknowledge_signature_invalidation,
                },
            ),
        ),
        Commands::Apply {
            config,
            output,
            format,
            in_place,
            confirm_in_place,
            allow_signed_input,
            acknowledge_signature_invalidation,
        } => (
            "apply",
            format,
            context.apply(
                &config,
                output.as_deref(),
                format,
                commands::AuthorizationFlags {
                    in_place,
                    confirm_in_place,
                    allow_signed_input,
                    acknowledge_signature_invalidation,
                },
            ),
        ),
        Commands::Verify {
            input,
            config,
            format,
        } => (
            "verify",
            format,
            context.verify(&input, config.as_deref(), format),
        ),
        Commands::ConvertIcon {
            input,
            output,
            fit,
            allow_crop,
            format,
        } => (
            "convert-icon",
            format,
            context.convert_icon(
                &input,
                &output,
                fit == IconFitArg::Cover,
                allow_crop,
                format,
            ),
        ),
    };

    match result {
        Ok(rendered) => {
            println!("{rendered}");
        }
        Err(error) => {
            if format == Format::Json {
                println!("{}", commands::render_error(command_name, &error, format));
            } else {
                eprintln!("{}", error.message());
            }
            std::process::exit(2);
        }
    }
}
