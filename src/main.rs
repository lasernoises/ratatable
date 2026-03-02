use std::path::PathBuf;

use clap::Parser;
use color_eyre::eyre::Result;
use ratatable::table::table;
use wraptatui::run;

#[derive(Parser)]
struct Cli {
    path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let path = cli.path;

    let mut data = if let Some(path) = path {
        ratatable::database_views::State::load(&path)?
    } else {
        Default::default()
    };

    run(&mut |p| {
        table(p, &mut data, || {
            Box::new(ratatable::database_views::MainView {})
        })
    })?;

    Ok(())
}
