use ruin_rs::{Interpreter, InterpreterConfig, Scope};
use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;

/// A Write wrapper that captures output to a Vec<u8>
struct CaptureWriter {
    buffer: Rc<RefCell<Vec<u8>>>,
}

impl Clone for CaptureWriter {
    fn clone(&self) -> Self {
        CaptureWriter {
            buffer: Rc::clone(&self.buffer),
        }
    }
}

impl CaptureWriter {
    fn new() -> Self {
        CaptureWriter {
            buffer: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn get_output(&self) -> String {
        let buffer = self.buffer.borrow();
        String::from_utf8_lossy(&buffer).to_string()
    }
}

impl Write for CaptureWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buffer.borrow_mut().flush()
    }
}

/// Categorize an error result as PARSE_ERROR or RUNTIME_ERROR
fn categorize_error(error: &str) -> &'static str {
    // Heuristic: errors mentioning tokens, grammar, or expected tokens are parse errors
    if error.contains("Expected")
        || error.contains("Unexpected")
        || error.contains("token")
        || error.contains("invalid token")
    {
        "PARSE_ERROR"
    } else {
        "RUNTIME_ERROR"
    }
}

/// Execute a script with a custom output buffer and return (result_category, output)
fn execute_with_capture(script: &str) -> (String, String) {
    // Create a capture writer
    let capture = CaptureWriter::new();
    let config = InterpreterConfig::with_output(Box::new(capture.clone()));

    let mut scope = Scope::default();
    let result = {
        use ruin_rs::lexer;
        use ruin_rs::parser::Parser;

        let tokens_result = lexer::lex(script);
        match tokens_result {
            Err(e) => Err(e),
            Ok(tokens) => {
                let parse_result = Parser::new(tokens).parse_program();
                match parse_result {
                    Err(e) => Err(e),
                    Ok(program) => {
                        let mut interpreter = Interpreter::new_with_config(&mut scope, config);
                        interpreter.execute_program(&program)
                    }
                }
            }
        }
    };

    // Get captured output
    let captured_output = capture.get_output();

    // Determine result category and format output
    let (category, expected_output) = match result {
        Ok(value) => {
            let value_repr = format!("{:#?}", value);
            let output = if captured_output.is_empty() {
                value_repr
            } else {
                format!("{}\nstdout: {}", value_repr, captured_output.trim())
            };
            ("OK".to_string(), output)
        }
        Err(error) => {
            let category = categorize_error(&error);
            (category.to_string(), error)
        }
    };

    (category, expected_output)
}

/// Compare expected and actual results, returning a nice error message if they differ
fn compare_results(
    test_name: &str,
    expected_category: &str,
    expected_output: &str,
    actual_category: &str,
    actual_output: &str,
) -> Result<(), String> {
    if expected_category != actual_category {
        return Err(format!(
            "{}: Category mismatch\n  Expected: {}\n  Actual: {}",
            test_name, expected_category, actual_category
        ));
    }

    if expected_output.trim() != actual_output.trim() {
        return Err(format!(
            "{}: Output mismatch\n  Expected:\n{}\n  Actual:\n{}",
            test_name, expected_output, actual_output
        ));
    }

    Ok(())
}

#[test]
fn run_ui_tests() {
    let ui_dir = Path::new("tests/ui");

    if !ui_dir.exists() {
        println!("No tests/ui directory found");
        return;
    }

    let mut test_files: Vec<_> = fs::read_dir(ui_dir)
        .expect("Failed to read tests/ui directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "ruin") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    test_files.sort(); // Run tests in stable order

    let mut failed_tests = Vec::new();
    let mut passed_tests = Vec::new();

    for test_file in test_files {
        let test_name = test_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let expected_file = test_file.with_extension("expected");

        // Read script file
        let script = match fs::read_to_string(&test_file) {
            Ok(content) => content,
            Err(e) => {
                failed_tests.push(format!("{}: Failed to read test file: {}", test_name, e));
                continue;
            }
        };

        // Read expected result file
        let expected_content = match fs::read_to_string(&expected_file) {
            Ok(content) => content,
            Err(e) => {
                failed_tests.push(format!(
                    "{}: Failed to read expected file: {}",
                    test_name, e
                ));
                continue;
            }
        };

        // Parse expected file: first line is category, rest is output
        let mut lines = expected_content.lines();
        let expected_category = match lines.next() {
            Some(cat) => cat.trim(),
            None => {
                failed_tests.push(format!("{}: Expected file is empty", test_name));
                continue;
            }
        };

        let expected_output = lines.collect::<Vec<_>>().join("\n");

        // Execute script with captured output
        let (actual_category, actual_output) = execute_with_capture(&script);

        // Compare results
        match compare_results(
            test_name,
            expected_category,
            &expected_output,
            &actual_category,
            &actual_output,
        ) {
            Ok(()) => {
                passed_tests.push(test_name.to_string());
                println!("✓ {}", test_name);
            }
            Err(e) => {
                failed_tests.push(e);
                println!("✗ {}", test_name);
            }
        }
    }

    // Print summary
    println!(
        "\n{} passed, {} failed",
        passed_tests.len(),
        failed_tests.len()
    );

    if !failed_tests.is_empty() {
        println!("\nFailed tests:");
        for failure in &failed_tests {
            println!("{}", failure);
        }
        panic!("{} UI tests failed", failed_tests.len());
    }
}
