mod cli;
mod error;
mod inference;
mod output;
mod privacy;
mod readers;
mod schema;
mod stats;
mod types;

use clap::Parser;
use cli::{Cli, Commands};
use error::Error;
use types::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Scan {
            input,
            out,
            k,
            bucket_counts,
            exact_counts,
            exact_median,
            hash_file,
            relaxed,
        }) => {
            let options = types::ProcessingOptions {
                k_anonymity: k,
                bucket_counts,
                exact_counts: exact_counts && relaxed,
                exact_median: exact_median && relaxed,
                hash_file,
                relaxed,
            };

            let extraction_result = schema::extract_schema(&input, options)?;

            // Write sidekick recode file if any recoding was done
            if let Some(ref sidekick_content) = extraction_result.recode_sidekick {
                let sidekick_path = input.with_extension("recode.txt");
                std::fs::write(&sidekick_path, sidekick_content)?;
                eprintln!("Recode mapping written to: {}", sidekick_path.display());
            }

            if let Some(out_path) = out {
                output::write_json_file(&extraction_result.manifest, &out_path)?;
                eprintln!("Manifest written to: {}", out_path.display());
            } else {
                output::write_json_stdout(&extraction_result.manifest)?;
            }
        }
        Some(Commands::Gui) | None => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                run_gui()?;
            }
            #[cfg(target_arch = "wasm32")]
            {
                eprintln!("GUI not supported on this platform");
            }
        }
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn run_gui() -> Result<()> {
    use crate::cli::GuiApp;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "ERT Manifest",
        options,
        Box::new(|_cc| Box::new(GuiApp::default())),
    )
    .map_err(|e| Error::InvalidInput(format!("GUI error: {}", e)))?;

    Ok(())
}
