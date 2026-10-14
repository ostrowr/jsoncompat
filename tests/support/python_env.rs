use std::path::PathBuf;
use std::process::Command;

#[allow(dead_code)]
pub fn python_command() -> Command {
    let mut command = Command::new("uv");
    configure_uv_environment(&mut command);
    configure_utf8_python_io(&mut command);
    command
        .arg("run")
        .arg("--no-config")
        .arg("--project")
        .arg(repo_pybindings_path())
        .arg("--all-extras")
        .arg("--locked");
    command
        .arg("python")
        .env("JSONCOMPAT_NATIVE_PROFILE", "debug")
        .env(
            "JSONCOMPAT_TEST_ARBITRARY_PRECISION",
            if cfg!(feature = "arbitrary-precision") {
                "1"
            } else {
                "0"
            },
        );
    add_repo_python_path(&mut command);
    command
}

#[allow(dead_code)]
pub fn ty_command() -> Command {
    let mut command = Command::new("uv");
    configure_uv_environment(&mut command);
    configure_utf8_python_io(&mut command);
    command
        .arg("run")
        .arg("--no-config")
        .arg("--project")
        .arg(repo_pybindings_path())
        .arg("--all-extras")
        .arg("--locked");
    command
        .arg("--with")
        .arg("ty==0.0.86")
        .arg("ty")
        .arg("check")
        .arg("--python-version")
        .arg("3.12")
        .arg("--extra-search-path")
        .arg(repo_pybindings_path())
        .arg("--error")
        .arg("all")
        .arg("--color")
        .arg("never");
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

fn configure_utf8_python_io(command: &mut Command) -> &mut Command {
    command
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
}

fn configure_uv_environment(command: &mut Command) -> &mut Command {
    for name in [
        "VIRTUAL_ENV",
        "UV_DEFAULT_INDEX",
        "UV_INDEX",
        "UV_INDEX_URL",
        "UV_EXTRA_INDEX_URL",
    ] {
        command.env_remove(name);
    }
    command
}

fn repo_pybindings_path() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("pybindings")
}
