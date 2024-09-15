use std::io::{Result, Write};

use log::info;

use super::collect_versions;
use crate::{codegen::general, env::Env, file_saver::save_to_file};

pub fn generate(env: &Env) {
    info!(
        "Generating sys build script for {}",
        env.config.library_name
    );

    let split_build_rs = env.config.split_build_rs;
    let path = env.config.target_path.join("build.rs");

    if !split_build_rs || !path.exists() {
        info!("Generating file {:?}", path);
        save_to_file(&path, env.config.make_backup, |w| {
            generate_build_script(w, env, split_build_rs)
        });
    }

    if split_build_rs {
        let path = env.config.target_path.join("build_version.rs");
        info!("Generating file {:?}", path);
        save_to_file(&path, env.config.make_backup, |w| {
            generate_build_version(w, env)
        });
    }
}

#[allow(clippy::write_literal)]
fn generate_build_script(w: &mut dyn Write, env: &Env, split_build_rs: bool) -> Result<()> {
    if !split_build_rs {
        general::start_comments(w, &env.config)?;
        writeln!(w)?;
    }

    let scripts = find_custom_build_scripts(env).unwrap_or_default();
    for script in &scripts {
        writeln!(w, "mod {};", script)?;
    }
    if !scripts.is_empty() {
        writeln!(w)?;
    }

    writeln!(
        w,
        r#"#[cfg(not(docsrs))]
use std::process;"#
    )?;

    if split_build_rs {
        writeln!(w)?;
        writeln!(w, "mod build_version;")?;
    }

    write!(
        w,
        "{}",
        r#"
#[cfg(docsrs)]
fn main() {} // prevent linking libraries to avoid documentation failure

#[cfg(not(docsrs))]
fn main() {
    if let Err(s) = system_deps::Config::new().probe() {
        println!("cargo:warning={s}");
        process::exit(1);
    }

    "#
    )?;

    for script in &scripts {
        writeln!(w, "{}::main();", script)?;
    }

    write!(
        w,
        "
}}
"
    )
}

fn find_custom_build_scripts(env: &Env) -> Result<Vec<String>> {
    let mut vec = Vec::<String>::new();
    for entry in std::fs::read_dir(env.config.auto_path.parent().unwrap())? {
        let path = entry?.path();
        let Some(ext) = path.extension() else {
            continue;
        };
        if ext != "rs" {
            continue;
        }
        let file_stem = path.file_stem().expect("No file name");
        let file_stem = file_stem
            .to_str()
            .expect("Can't convert file name to string")
            .to_owned();

        if !file_stem.starts_with("build_") {
            continue;
        }

        vec.push(file_stem);
    }
    vec.sort();

    Ok(vec)
}

fn generate_build_version(w: &mut dyn Write, env: &Env) -> Result<()> {
    general::start_comments(w, &env.config)?;
    writeln!(w)?;
    writeln!(w, "pub fn version() -> &'static str {{")?;
    write_version(w, env, false)?;
    writeln!(w, "}}")
}

fn write_version(w: &mut dyn Write, env: &Env, for_let: bool) -> Result<()> {
    let versions = collect_versions(env);

    for (version, lib_version) in versions.iter().rev() {
        write!(
            w,
            "if cfg!({}) {{\n\t\t\"{}\"\n\t}} else ",
            version.to_cfg(None),
            lib_version
        )?;
    }
    let end = if for_let { ";" } else { "" };
    writeln!(w, "{{\n\t\t\"{}\"\n\t}}{}", env.config.min_cfg_version, end)
}
