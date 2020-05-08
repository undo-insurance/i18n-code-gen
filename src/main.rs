#[doc(inline)]
pub use std;

mod code_gen;
mod lokalise_client;
mod scala_ast;

use anyhow::{Error, Result};
use code_gen::generate_code;
use crossterm::cursor::{Hide, MoveTo, RestorePosition, SavePosition, Show};
use crossterm::{
    execute,
    style::Print,
    terminal::{Clear, ClearType},
};
use lokalise_client::{LokaliseClient, Project};
use std::io::{self, Write};
use tokio::task;
use tokio::time::delay_for;
use tokio::time::Duration;

#[tokio::main]
async fn main() {
    let task = task::spawn(async {
        ctrlc::set_handler(move || {
            execute!(io::stderr(), Show).ok();
            std::process::exit(1);
        })
        .expect("Error setting Ctrl-C handler");

        show_spinner();

        let client = LokaliseClient::new();

        let project = find_undo_project(&client).await?;
        let keys = client.keys(&project).await?;

        let code = generate_code(keys)?;
        println!("{}", code);

        Result::<_>::Ok(())
    });

    match task.await {
        Ok(Ok(())) => {
            execute!(io::stderr(), Show).ok();
        }
        Ok(Err(err)) => {
            execute!(io::stderr(), Show).ok();

            eprintln!("{}", err);
            std::process::exit(1);
        }
        Err(err) => {
            execute!(io::stderr(), Show).ok();

            eprintln!("Error joining task: {}", err);
            std::process::exit(1);
        }
    }
}

async fn find_undo_project(client: &LokaliseClient) -> Result<Project> {
    let project = client
        .projects()
        .await?
        .into_iter()
        .find(|project| project.name == "Undo")
        .ok_or_else(|| Error::msg("Couldn't find Undo project"))?;
    Ok(project)
}

#[allow(unreachable_code)]
fn show_spinner() {
    task::spawn(async move {
        let states = ["|-    |", "| -   |", "|  -  |", "|   - |", "|    -|"];

        let mut state = 0;
        let mut up = true;
        let mut delay = 80;

        loop {
            execute!(
                io::stderr(),
                Hide,
                SavePosition,
                Print(states[state]),
                RestorePosition,
            )?;

            match (up, state) {
                (_, 0) => {
                    state += 1;
                    up = true;
                }

                (true, 1) | (true, 2) | (true, 3) => {
                    state += 1;
                }
                (false, 1) | (false, 2) | (false, 3) => {
                    state -= 1;
                }

                (_, 4) => {
                    state -= 1;
                    up = false;
                }
                _ => panic!("invalid state {}", state),
            }

            delay_for(Duration::from_millis(delay)).await;
            delay = delay.checked_sub(1).unwrap_or(1).max(20);

            execute!(io::stderr(), Clear(ClearType::CurrentLine))?;
        }
        Result::<_>::Ok(())
    });
}
