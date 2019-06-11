// build.rs

extern crate bindgen;
#[cfg(unix)]
extern crate pkg_config;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/* TODO: Create an abstraction layer for Meson and Ninja */

const REPO: &'static str = "https://code.videolan.org/videolan/dav1d.git";
const OUTPUT_DIR: &'static str = "target/dav1d";

macro_rules! runner {
    ($name:expr) => { run($name, |command| {command}); };
    ($name:expr, $($arg:expr),*) => {
        run($name, |command| {
                command
                $(.arg($arg))*
        });
    };
}

fn run<F>(name: &str, mut exec: F)
where
    F: FnMut(&mut Command) -> &mut Command,
{
    let mut command = Command::new(name);
    let executed = exec(&mut command);
    if !executed.status().unwrap().success() {
        panic!("Failed to execute {:?}", executed);
    }
}

fn get_dav1d(path: &PathBuf) {
    let repo_path = path.join(".git");
    if !Path::new(&repo_path).exists() {
        runner!("git", "clone", "--depth=1", REPO, &path);
    } else {
        runner!(
            "git",
            format!("--git-dir={}", repo_path.to_str().unwrap()),
            format!("--work-tree={}", path.to_str().unwrap()),
            "pull"
        );
    }
}

fn run_meson(path: &PathBuf, build_path: &PathBuf, release_dir: &str) {
    let release_path = path.parent().unwrap().join(release_dir);
    runner!(
        "meson",
        "setup",
        "-Ddefault_library=static",
        format!("--prefix={}", &release_path.to_str().unwrap()),
        &build_path.to_str().unwrap(),
        &path.to_str().unwrap()
    );
    runner!("ninja", format!("-C{}", build_path.to_str().unwrap()));
    runner!("meson", "install", format!("-C{}", build_path.to_str().unwrap()));
}

fn main() {
    let source = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join(OUTPUT_DIR);
    let release_dir = "release";

    let build_dir = "build";
    let build_path = source.parent().unwrap().join(build_dir);

    get_dav1d(&source);

    run_meson(&source, &build_path, &release_dir);

    // Set pkg-config path
    let key = "PKG_CONFIG_PATH";
    let value = format!("{}/{}", build_path.to_str().unwrap(), "meson-private");
    env::set_var(key, &value);

    let libs = pkg_config::Config::new().probe("dav1d").unwrap();

    let headers = libs.include_paths.clone();

    let mut builder = bindgen::builder()
        .blacklist_type("max_align_t")
        .rustfmt_bindings(false)
        .header(headers[0].join("dav1d/dav1d.h").to_str().unwrap());

    for header in headers {
        builder = builder.clang_arg("-I").clang_arg(header.to_str().unwrap());
    }

    // Manually fix the comment so rustdoc won't try to pick them
    let s = builder
        .generate()
        .unwrap()
        .to_string()
        .replace("/**", "/*")
        .replace("/*!", "/*");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut file = File::create(out_path.join("dav1d.rs")).unwrap();

    let _ = file.write(s.as_bytes());
}
