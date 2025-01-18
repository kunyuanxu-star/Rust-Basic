use std::process::{Command, exit};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use serde::{Serialize, Deserialize};
use std::time::Instant;

#[derive(Serialize, Deserialize, Debug)]
struct ExerciseResult {
    name: String,
    result: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Statistics {
    total_exercations: usize,
    total_succeeds: usize,
    total_failures: usize,
    total_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Report {
    exercises: Vec<ExerciseResult>,
    user_name: Option<String>,
    statistics: Statistics,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let exercises_dir = "exercises";

    if args.len() < 2 {
        eprintln!("Please provide a command: 'watch' or 'all'");
        exit(1);
    }

    let mode = &args[1]; // 'watch' or 'all'
    let start_time = Instant::now(); // 记录开始时间

    // 扫描 exercises 目录，获取所有的直接子目录和文件
    let exercise_dirs = match scan_directory(exercises_dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Error scanning exercises directory: {}", e);
            exit(1);
        }
    };

    let mut report = Report {
        exercises: Vec::new(),
        user_name: None,
        statistics: Statistics {
            total_exercations: 0,
            total_succeeds: 0,
            total_failures: 0,
            total_time: 0,
        },
    };

    // 根据模式选择执行逐题评测或一次性评测
    if mode == "watch" {
        // 逐题评测
        for exercise_dir in exercise_dirs {
            if exercise_dir.is_dir() {
                let name = exercise_dir.display().to_string();
                if exercise_dir.join("Cargo.toml").exists() {
                    // 如果目录下有 Cargo.toml 文件，认为这是一个完整的 Cargo 项目
                    println!("\nEvaluating Cargo project: {}", name);
                    let result = evaluate_cargo_project(&exercise_dir);
                    print_evaluation_result(&name, result);
                    report.exercises.push(ExerciseResult { name, result });
                    if result {
                        report.statistics.total_succeeds += 1;
                    } else {
                        report.statistics.total_failures += 1;
                    }
                } else {
                    // 如果目录下没有 Cargo.toml 文件，则认为目录中的每个 .rs 文件都是单文件习题
                    let rs_files = get_rs_files_in_directory(&exercise_dir);
                    for rs_file in rs_files {
                        let file_name = rs_file.display().to_string();
                        println!("\nEvaluating single file: {}", file_name);
                        let result = evaluate_single_file(&rs_file);
                        print_evaluation_result(&file_name, result);
                        report.exercises.push(ExerciseResult { name: file_name, result });
                        if result {
                            report.statistics.total_succeeds += 1;
                        } else {
                            report.statistics.total_failures += 1;
                        }
                        // 打印详细的编译器输出和cargo test输出
                        print_compiler_output(&rs_file);
                        print_cargo_test_output(&rs_file);
                        // 在每个文件评测结束后，等待用户输入以进行下一道题目
                        if !ask_to_continue() {
                            break;
                        }
                    }
                }
            }
        }
    } else if mode == "all" {
        // 一次性评测所有题目
        for exercise_dir in exercise_dirs {
            if exercise_dir.is_dir() {
                let name = exercise_dir.display().to_string();
                if exercise_dir.join("Cargo.toml").exists() {
                    // 如果目录下有 Cargo.toml 文件，认为这是一个完整的 Cargo 项目
                    println!("\nEvaluating Cargo project: {}", name);
                    let result = evaluate_cargo_project(&exercise_dir);
                    print_evaluation_result(&name, result);
                    report.exercises.push(ExerciseResult { name, result });
                    if result {
                        report.statistics.total_succeeds += 1;
                    } else {
                        report.statistics.total_failures += 1;
                    }
                } else {
                    // 如果目录下没有 Cargo.toml 文件，则认为目录中的每个 .rs 文件都是单文件习题
                    let rs_files = get_rs_files_in_directory(&exercise_dir);
                    for rs_file in rs_files {
                        let file_name = rs_file.display().to_string();
                        println!("\nEvaluating single file: {}", file_name);
                        let result = evaluate_single_file(&rs_file);
                        print_evaluation_result(&file_name, result);
                        report.exercises.push(ExerciseResult { name: file_name, result });
                        if result {
                            report.statistics.total_succeeds += 1;
                        } else {
                            report.statistics.total_failures += 1;
                        }
                    }
                }
            }
        }
    } else {
        eprintln!("Invalid command. Please use 'watch' or 'all'.");
        exit(1);
    }

    // 修正统计，total_exercations 为通过题目 + 失败题目
    report.statistics.total_exercations = report.statistics.total_succeeds + report.statistics.total_failures;

    // 计算总时间
    report.statistics.total_time = start_time.elapsed().as_secs(); // 评测结束时间 - 开始时间

    // 清理 exercises 目录下的所有 target 目录
    if let Err(e) = clean_target_dirs(exercises_dir) {
        eprintln!("Error cleaning target directories: {}", e);
    }

    // 输出总结信息
    println!("\nSummary:");
    println!("Total exercises: {}", report.statistics.total_exercations);
    println!("Total successes: {}", report.statistics.total_succeeds);
    println!("Total failures: {}", report.statistics.total_failures);

    // 保存评测结果到 JSON 文件
    if let Err(e) = save_report_to_json("report.json", &report) {
        eprintln!("Error saving report to JSON file: {}", e);
    }
}

// 扫描目录并返回其直接子目录（不递归）
fn scan_directory<P: AsRef<Path>>(dir: P) -> Result<Vec<PathBuf>, io::Error> {
    let mut result = Vec::new();
    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // 如果是目录，直接添加到结果列表
            result.push(path);
        }
    }

    Ok(result)
}

// 获取目录下所有的 .rs 文件
fn get_rs_files_in_directory<P: AsRef<Path>>(dir: P) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
                    result.push(path);
                }
            }
        }
    }
    result
}

// 评测完整的 Cargo 项目
fn evaluate_cargo_project(exercise_dir: &PathBuf) -> bool {
    let build_result = run_cargo_command(exercise_dir, "build");
    let test_result = run_cargo_command(exercise_dir, "test");
    let clippy_result = run_cargo_command(exercise_dir, "clippy");

    build_result && test_result && clippy_result
}

// 评测单文件习题
fn evaluate_single_file(exercise_file: &PathBuf) -> bool {
    run_rustc_command(exercise_file).is_ok()
}

// 运行 rustc 编译并执行单文件习题
fn run_rustc_command(exercise_file: &PathBuf) -> Result<(), String> {
    let output = Command::new("rustc")
        .arg(exercise_file)
        .output()
        .map_err(|e| format!("Failed to execute rustc: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rustc compilation failed: {}", stderr));
    }

    // 执行编译后的文件
    let compiled_file = exercise_file.with_extension(""); // 生成编译后的可执行文件路径
    let output = Command::new(compiled_file)
        .output()
        .map_err(|e| format!("Failed to execute compiled file: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Execution failed: {}", stderr));
    }

    Ok(())
}

// 运行 cargo 命令（如 build, test, clippy 等）
fn run_cargo_command(exercise_dir: &PathBuf, command: &str) -> bool {
    let output = Command::new("cargo")
        .arg(command)
        .current_dir(exercise_dir)
        .output()
        .map_err(|e| format!("Failed to execute cargo {}: {}", command, e));

    match output {
        Ok(output) => {
            if !output.stdout.is_empty() {
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
            if !output.stderr.is_empty() {
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
            output.status.success()
        },
        Err(_) => false,
    }
}

// 打印每道题目的评测结果，并使用颜色输出
fn print_evaluation_result(name: &str, result: bool) {
    if result {
        println!("\x1b[32m{}: PASSED\x1b[0m", name); // 绿色表示成功
    } else {
        println!("\x1b[31m{}: FAILED\x1b[0m", name); // 红色表示失败
    }
}

// 提示用户是否继续评测下一题
fn ask_to_continue() -> bool {
    let mut input = String::new();
    println!("\nPress any key to continue, or 'q' to quit.");
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_lowercase() != "q"
}

// 打印编译器输出
fn print_compiler_output(exercise_file: &PathBuf) {
    let output = Command::new("rustc")
        .arg(exercise_file)
        .output()
        .expect("Failed to execute rustc");
    println!("Compiler Output for {}: \n{}", exercise_file.display(), String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprintln!("Compiler Errors for {}: \n{}", exercise_file.display(), String::from_utf8_lossy(&output.stderr));
    }
}

// 打印 cargo test 输出
fn print_cargo_test_output(exercise_file: &PathBuf) {
    let output = Command::new("cargo")
        .arg("test")
        .current_dir(exercise_file.parent().unwrap())
        .output()
        .expect("Failed to execute cargo test");
    println!("Cargo Test Output for {}: \n{}", exercise_file.display(), String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprintln!("Cargo Test Errors for {}: \n{}", exercise_file.display(), String::from_utf8_lossy(&output.stderr));
    }
}

// 清理 exercises 目录下的所有 target 目录
fn clean_target_dirs<P: AsRef<Path>>(base_dir: P) -> Result<(), io::Error> {
    let entries = fs::read_dir(base_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // 如果是目录，检查是否包含 target 目录
            let target_dir = path.join("target");
            if target_dir.exists() {
                fs::remove_dir_all(target_dir)?;
                println!("Successfully cleaned target directory in: {}", path.display());
            }
        }
    }

    Ok(())
}

// 保存评测结果到 JSON 文件
fn save_report_to_json(file_name: &str, report: &Report) -> io::Result<()> {
    let file = File::create(file_name)?;
    serde_json::to_writer_pretty(file, report)?;
    Ok(())
}
