mod code_gen;
mod lokalise_client;
mod scala_ast;

use anyhow::{Error, Result};
use code_gen::generate_code;
use crossterm::{
    cursor::{Hide, RestorePosition, SavePosition, Show},
    execute,
    style::Print,
    terminal::{Clear, ClearType},
};
use lokalise_client::{LokaliseClient, Project};
use std::ffi::OsStr;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::exit,
};
use tokio::stream::StreamExt;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    runtime::Runtime,
    task,
    time::{delay_for, Duration},
};

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

    let path = path_to_write_to().await?.join("shared/src/main/scala/dk/undo/i18n/I18n.scala");
    let mut file = File::create(path).await?;
    file.write_all(code.as_bytes()).await?;

    Result::<_>::Ok(())
}

async fn path_to_write_to() -> Result<PathBuf> {
    for path in std::env::current_dir()?.ancestors() {
        if is_root_of_backend(path).await? {
            return Ok(PathBuf::from(path));
        }
    }

    Err(Error::msg(
        "Could not find root of backend project from parents of current dir",
    ))
}

async fn is_root_of_backend(path: &Path) -> Result<bool> {
    let mut dir_conents = fs::read_dir(path).await?;

    let mut contains_git = false;
    let mut contains_build_sbt = false;

    let git_file = Some(OsStr::new(".git"));
    let build_sbt_file = Some(OsStr::new("build.sbt"));

    while let Some(path_in_dir) = dir_conents.next().await {
        let path_in_dir = path_in_dir?.path();

        if path_in_dir.file_name() == git_file {
            contains_git = true
        }

        if path_in_dir.file_name() == build_sbt_file {
            contains_build_sbt = true;
        }
    }

    Ok(contains_git && contains_build_sbt)
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
