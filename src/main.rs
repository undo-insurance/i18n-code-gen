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
use structopt::StructOpt;
use tokio::stream::StreamExt;
use tokio::{
    fs::{self, File},
    io::{AsyncWrite, AsyncWriteExt},
    runtime::Runtime,
    task,
    time::{delay_for, Duration},
};

/// Download translations from Lokalise and generate Scala code.
#[derive(Debug, StructOpt)]
struct Opt {
    /// Print the code to stdout rather than a file.
    #[structopt(long = "stdout", short = "s")]
    print_to_stdout: bool,

    /// Your Lokalise API token.
    ///
    /// If not set it'll use the `LOKALISE_API_TOKEN` environment variable.
    #[structopt(long = "token", short = "t")]
    api_token: Option<String>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let result = std::panic::catch_unwind(|| {
        Runtime::new()?.block_on(async_main(opt))?;
        Result::<_>::Ok(())
    });

    execute!(io::stderr(), Show).ok();

    match result {
        Ok(ok) => match ok {
            Ok(()) => {}
            Err(err) => {
                eprintln!("{}", err);
                eprintln!("{}", err.backtrace());
                exit(1);
            }
        },
        Err(_panic_err) => {
            exit(1);
        }
    }

    Ok(())
}

async fn async_main(opt: Opt) -> Result<()> {
    ctrlc::set_handler(move || {
        execute!(io::stderr(), Show).ok();
        std::process::exit(1);
    })
    .expect("Error setting Ctrl-C handler");

    show_spinner();

    if opt.print_to_stdout {
        let stdout = tokio::io::stdout();
        gen_code_and_write_to(opt, stdout).await?;
    } else {
        let path = path_to_write_to()
            .await?
            .join("shared/src/main/scala/dk/undo/i18n/I18n.scala");
        let file = File::create(path).await?;
        gen_code_and_write_to(opt, file).await?;
    }

    Ok(())
}

async fn gen_code_and_write_to<W>(opt: Opt, mut out: W) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let api_token = if let Some(api_token) = opt.api_token {
        api_token
    } else {
        std::env::var("LOKALISE_API_TOKEN").expect("LOKALISE_API_TOKEN is not set")
    };

    let client = LokaliseClient::new(api_token);

    // Lokalise's API doesn't support concurrent requests using the same API token...
    // So don't bother making these requests in parallel.

    let projects = vec!["Undo", "Car"];

    let mut project_and_keys = Vec::new();
    for name in projects {
        let project = find_project(name, &client).await?;
        let keys = client.keys(&project).await?;
        project_and_keys.push((project, keys));
    }

    let code = generate_code(project_and_keys)?;
    execute!(io::stderr(), Clear(ClearType::CurrentLine))?;

    out.write_all(code.as_bytes()).await?;

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

async fn find_project(name: &str, client: &LokaliseClient) -> Result<Project> {
    let project = client
        .projects()
        .await?
        .into_iter()
        .find(|project| project.name == name)
        .ok_or_else(|| Error::msg(format!("Couldn't find {} project", name)))?;
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
