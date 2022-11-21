use anyhow::{Error, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{
    fmt::Display,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    compiler::Compiler,
    json::{Script, ScriptResult, Test, TestDefinition},
};

pub fn run_test_definition(
    test_definition: TestDefinition,
    scripts_dir: PathBuf,
    compiler: Box<dyn Compiler + Send>,
) -> Result<()> {
    println!("{}", test_definition.description);

    for test in &test_definition.tests {
        if let Err(err) = run_test(test, &scripts_dir, &compiler) {
            println!("{}", err);
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
struct FailedScript {
    expected: ScriptResult,
    actual: ScriptResult,
    output: String,
    path: PathBuf,
}

impl Display for FailedScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Script failed: {} (expected: {} actual: {})\n{}",
            self.path.display(),
            self.expected,
            self.actual,
            self.output
        )
    }
}

fn run_test<P: AsRef<Path>>(
    test: &Test,
    scripts_dir: P,
    compiler: &Box<dyn Compiler + Send>,
) -> Result<()> {
    let available_parallelism = thread::available_parallelism()
        .map(|x| x.get())
        .unwrap_or(1_usize);

    let chunk_size = {
        let chunk_size = test.scripts.len() / available_parallelism;
        if chunk_size == 0 {
            1_usize
        } else {
            chunk_size
        }
    };

    let chunks: Vec<&[Script]> = test.scripts.chunks(chunk_size).collect();
    let mut handles: Vec<JoinHandle<_>> = Vec::with_capacity(chunk_size);

    let (tx_errors, rx_errors): (Sender<Vec<Error>>, Receiver<Vec<Error>>) = mpsc::channel();
    let (tx_failed_scripts, rx_failed_scripts): (
        Sender<Vec<FailedScript>>,
        Receiver<Vec<FailedScript>>,
    ) = mpsc::channel();

    let multi_progress = MultiProgress::new();
    let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");

    for chunk in chunks {
        let thread_tx_errors = tx_errors.clone();
        let thread_tx_failed_scripts = tx_failed_scripts.clone();

        let chunk = chunk.to_vec();
        let compiler: Box<dyn Compiler + Send> = dyn_clone::clone_box(compiler.as_ref());
        let scripts_dir = scripts_dir.as_ref().to_owned();

        let pb = multi_progress.add(ProgressBar::new(chunk.len() as u64));
        pb.set_style(spinner_style.clone());

        handles.push(thread::spawn(move || {
            chunk_thread(
                pb,
                thread_tx_errors,
                thread_tx_failed_scripts,
                chunk,
                compiler,
                scripts_dir,
            );
        }));
    }

    let mut thread_results = vec![];
    let mut thread_chunk_errors = vec![];
    let mut thread_chunk_failed_scripts = vec![];

    for handle in handles {
        thread_results.push(handle.join());
        thread_chunk_errors.push(rx_errors.recv_timeout(Duration::from_secs(1)));
        thread_chunk_failed_scripts.push(rx_failed_scripts.recv_timeout(Duration::from_secs(1)));
    }

    if let Err(err) = multi_progress.clear() {
        println!("Unable to clear MultiProgressBar: {}", err);
    }

    // report the panics after joining all threads so we don't have any leaking and still running threads
    for thread_result in thread_results {
        thread_result.expect("Thread paniced!");
    }

    for chunk_errors in thread_chunk_errors.into_iter().flatten() {
        for err in chunk_errors {
            println!("{}", err);
        }
    }

    for chunk_failed_scripts in thread_chunk_failed_scripts.into_iter().flatten() {
        for failed_script in chunk_failed_scripts {
            println!("{}", failed_script);
        }
    }

    Ok(())
}

fn chunk_thread(
    pb: ProgressBar,
    tx_errors: Sender<Vec<Error>>,
    tx_failed_scripts: Sender<Vec<FailedScript>>,
    chunk: Vec<Script>,
    compiler: Box<dyn Compiler>,
    scripts_dir: PathBuf,
) {
    let mut failed_scripts = vec![];
    let mut errors = vec![];

    for script in chunk {
        pb.set_message(script.file.clone());
        pb.inc(1);

        match compile_script(&scripts_dir, &script, compiler.as_ref()) {
            Ok(result) => match result {
                None => {}
                Some(failed_script) => failed_scripts.push(failed_script),
            },
            Err(err) => errors.push(err),
        }
    }

    pb.finish_with_message("waiting");

    tx_errors.send(errors).expect("Unable to send errors");

    tx_failed_scripts
        .send(failed_scripts)
        .expect("Unable to send failed scripts");
}

fn compile_script<P: AsRef<Path>>(
    scripts_dir: P,
    script: &Script,
    compiler: &dyn Compiler,
) -> Result<Option<FailedScript>> {
    let script_path = scripts_dir.as_ref().join(&script.file);
    let compiler_result = compiler.compile(&script_path)?;

    let expected = script.expected_result;
    let actual = match compiler_result.success {
        true => ScriptResult::Success,
        false => ScriptResult::Failure,
    };

    if expected != actual {
        return Ok(Some(FailedScript {
            expected,
            actual,
            output: compiler_result.output,
            path: script_path,
        }));
    }

    Ok(None)
}
