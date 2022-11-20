use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::{fs, thread};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use json::TestDefinition;

use crate::args::Args;

mod args;
mod json;

fn main() {
    match run() {
        Ok(_) => {}
        Err(err) => println!("{:#?}", err),
    };
}

fn run() -> Result<()> {
    let args = Args::parse();

    if !args.definition.exists() {
        return Err(anyhow!(
            "File {} does not exist!",
            args.definition.display()
        ));
    }

    if !args.scripts_dir.exists() {
        return Err(anyhow!(
            "Directory {} does not exist!",
            args.scripts_dir.exists()
        ));
    }

    if !args.compiler_path.exists() {
        return Err(anyhow!(
            "File {} does not exist!",
            args.compiler_path.display()
        ));
    }

    let file_contents = fs::read_to_string(&args.definition)
        .with_context(|| format!("Unable to read file {}", args.definition.display()))?;

    let test_definition: json::TestDefinition = serde_json::from_str(&file_contents)
        .with_context(|| format!("Invalid JSON in {}", args.definition.display()))?;

    run_test_definition(test_definition, args.scripts_dir, args.compiler_path)
}

fn run_test_definition(
    test_definition: TestDefinition,
    scripts_dir: PathBuf,
    compiler_path: PathBuf,
) -> Result<()> {
    println!("------------ Test Definition ------------");
    println!("{}", test_definition.description);

    for test in test_definition.tests {
        run_test(&test, scripts_dir.clone(), compiler_path.clone());
    }

    Ok(())
}

fn run_test(test: &json::Test, scripts_dir: PathBuf, compiler_path: PathBuf) {
    let count = thread::available_parallelism()
        .map(|x| x.get())
        .unwrap_or(1_usize);

    let chunks: Vec<&[json::Script]> = test.scripts.chunks(count).collect();

    let multi_progress = MultiProgress::new();

    let mut handles = vec![];

    let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");

    for chunk in chunks {
        let scripts_dir = scripts_dir.clone();
        let compiler_path = compiler_path.clone();
        let chunk = chunk.to_vec();

        let pb = multi_progress.add(ProgressBar::new(chunk.len() as u64));
        pb.set_style(spinner_style.clone());

        handles.push(thread::spawn(move || {
            let scripts_dir = scripts_dir;
            let compiler_path = compiler_path;

            for script in chunk {
                pb.set_message(script.file.to_string());
                pb.inc(1);
                compile_script(&script, &scripts_dir, &compiler_path);
            }

            pb.finish_with_message("waiting...");
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    multi_progress.clear().unwrap();
}

fn compile_script(script: &json::Script, scripts_dir: &Path, compiler_path: &Path) {
    let script_path = scripts_dir.join(&script.file);
    if !script_path.exists() {
        println!("Script {} does not exist!", script_path.display());
        return;
    }

    let script_name = script_path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let import_dir = script_path.parent().unwrap().to_owned();

    let process_output = std::process::Command::new(compiler_path)
        .stdin(std::process::Stdio::null())
        .arg(script_name)
        .arg("-o=out")
        .arg(format!("-i={}", import_dir.display()))
        .output()
        .unwrap();

    let result_success = process_output.status.success();
    let expected_success = script.expected_result == json::ScriptResult::Success;
    let test_success = result_success == expected_success;

    if !test_success {
        println!("Test failed");
        io::stdout().write_all(&process_output.stdout).unwrap();
        io::stderr().write_all(&process_output.stderr).unwrap();
    }
}
