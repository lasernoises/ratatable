use std::path::PathBuf;

use clap::Parser;
use ratatable::{
    database::{self},
    table::table,
};
use wraptatui::{run, widgets::state::state};

#[derive(Parser)]
struct Cli {
    path: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let mut path = cli.path;

    run(&mut |p| {
        state(
            p,
            &mut path,
            |path| {
                if let Some(path) = path {
                    database::State::load(path)
                } else {
                    Default::default()
                }
            },
            |p, _, data: &mut database::State| {
                table(p, data, || Box::new(ratatable::database::MainView {}))
            },
        )
    })
    .unwrap();
}
