#[doc(inline)]
pub use std;

mod code_gen;
mod lokalise_client;
mod scala_ast;

use anyhow::{Error, Result};
use code_gen::generate_code;
use crossterm::cursor::{Hide, MoveTo, RestorePosition, SavePosition, Show};
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use crossterm::{
    execute,
    style::Print,
    terminal::{Clear, ClearType},
};
use lokalise_client::{LokaliseClient, Project};
use std::io::{self, Write};
use std::path::Path;
use std::process::exit;
use tokio::fs::File;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::task;
use tokio::time::delay_for;
use tokio::{runtime::Runtime, time::Duration};

fn main() -> Result<()> {
    let result = std::panic::catch_unwind(|| {
        Runtime::new()?.block_on(async_main())?;
        Result::<_>::Ok(())
    });

    execute!(io::stderr(), Show).ok();

    match result {
        Ok(ok) => match ok {
            Ok(()) => {}
            Err(err) => {
                eprintln!("{}", err);
                exit(1);
            }
        },
        Err(_panic_err) => {
            exit(1);
        }
    }

    Ok(())
}

async fn async_main() -> Result<()> {
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
    execute!(io::stderr(), Clear(ClearType::CurrentLine))?;

    let path = Path::new("shared/src/main/scala/dk/undo/i18n/I18n.scala");
    let mut file = File::create(path).await?;
    file.write_all(code.as_bytes()).await?;

    Result::<_>::Ok(())
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
        let states = ["|", "/", "-", "\\", "|", "/", "-", "\\"];

        let mut state = 1;
        let mut up = true;
        let delay = 80;

        loop {
            execute!(
                io::stderr(),
                Hide,
                SavePosition,
                Print(states[state - 1]),
                RestorePosition,
            )?;

            if state == 1 {
                state += 1;
                up = true;
            } else if state == states.len() {
                state -= 1;
                up = false;
            } else if up {
                state += 1;
            } else {
                state -= 1;
            }

            delay_for(Duration::from_millis(delay)).await;
            execute!(io::stderr(), Clear(ClearType::CurrentLine))?;
        }
        Result::<_>::Ok(())
    });
}
