use std::path::PathBuf;
use std::process::Command;

#[allow(dead_code)]
pub fn python_command() -> Command {
    let mut command = Command::new("uv");
    command.env_remove("VIRTUAL_ENV");
    command
        .arg("run")
        .arg("--project")
        .arg(repo_pybindings_path())
        .arg("python");
    add_repo_python_path(&mut command);
    command
}

#[allow(dead_code)]
pub fn pyright_command() -> Command {
    let mut command = Command::new("uv");
    command.env_remove("VIRTUAL_ENV");
    command
        .arg("run")
        .arg("--project")
        .arg(repo_pybindings_path())
        .arg("--with")
        .arg("pyright==1.1.408")
        .arg("pyright");
    add_repo_python_path(&mut command);
    command
}

pub fn add_repo_python_path(command: &mut Command) -> &mut Command {
    let mut paths = vec![repo_pybindings_path()];
    if let Some(existing) = std::env::var_os("PYTHONPATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    command.env(
        "PYTHONPATH",
        std::env::join_paths(paths).expect("build PYTHONPATH"),
    )
}

fn repo_pybindings_path() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("pybindings")
}
