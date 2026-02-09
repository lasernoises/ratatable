use wraptatui::{run, widgets::state::state};

use ratatable::{database::Database, table::table};

fn main() {
    run(&mut |p| {
        state(p, |p, data: &mut Database| {
            table(p, data, || Box::new(ratatable::database::MainView {}))
        })
    })
    .unwrap();
}
