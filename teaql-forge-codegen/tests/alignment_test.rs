use std::fs;
use std::path::Path;
use teaql_forge_codegen::context::build_render_context;
use teaql_forge_codegen::engine::{
    generate_virtual_crate, generate_virtual_workspace, GeneratedFile,
};
use teaql_forge_model::parser::parse_model;

#[test]
fn test_perfectly_valid_alignment() {
    let xml_content =
        fs::read_to_string("../../teaql-code-gen/ksml-test-cases/01_perfectly_valid.xml").unwrap();
    let domain = parse_model(&xml_content, "01_perfectly_valid.xml").unwrap();
    let render_domain = build_render_context(&domain);

    // 1. Test rust-lib-core (crate)
    let lib_files = match generate_virtual_crate(&render_domain) {
        Ok(files) => files,
        Err(e) => {
            println!("JINJA ERROR: {:#?}", e);
            panic!("Jinja error");
        }
    };
    for file in &lib_files {
        if file.path.ends_with("runtime.rs") {
            fs::write("runtime.rs.txt", &file.content).unwrap();
        }
    }
    assert_matches_expected(
        &lib_files,
        "tests/expected/01_perfectly_valid/rust-lib-core",
    );

    // 2. Test rust-app-console (workspace)
    let workspace_files = generate_virtual_workspace(&render_domain).unwrap();
    assert_matches_expected(
        &workspace_files,
        "tests/expected/01_perfectly_valid/rust-app-console",
    );
}

fn assert_matches_expected(generated_files: &[GeneratedFile], expected_dir: &str) {
    let expected_dir_path = Path::new(expected_dir);
    for file in generated_files {
        let expected_path = expected_dir_path.join(&file.path);
        assert!(
            expected_path.exists(),
            "Expected file not found: {:?}",
            expected_path
        );
        let expected_content = fs::read_to_string(expected_path).unwrap();
        let expected_content = {
            let lines: Vec<&str> = expected_content.lines().collect();
            let cleaned_lines: Vec<String> = lines
                .into_iter()
                .map(|l| l.trim_end().to_string())
                .collect();
            let mut out = cleaned_lines.join("\n");
            out = out.trim_start_matches('\n').to_string();
            out = out.trim_end_matches('\n').to_string();
            out
        };
        if file.content != expected_content {
            println!("Writing left.rs for {:?}", file.path);
            std::fs::write("left.rs", &file.content).unwrap();
            std::fs::write("right.rs", &expected_content).unwrap();
        }
        assert_eq!(
            file.content, expected_content,
            "Content mismatch for file: {:?}",
            file.path
        );
    }

    // Also assert that there are no extra files in the expected directory that were not generated
    let mut expected_paths = vec![];
    visit_dirs(expected_dir_path, &mut expected_paths);
    expected_paths.retain(|p| {
        let rel_path = p.strip_prefix(expected_dir_path).unwrap();
        if expected_dir_path.ends_with("rust-app-console") {
            !rel_path.starts_with("lib")
        } else {
            true
        }
    });

    let mut generated_paths = generated_files
        .iter()
        .map(|f| expected_dir_path.join(&f.path))
        .collect::<Vec<_>>();

    expected_paths.sort();
    generated_paths.sort();

    assert_eq!(
        expected_paths, generated_paths,
        "Mismatch in generated file set!"
    );
}

fn visit_dirs(dir: &Path, paths: &mut Vec<std::path::PathBuf>) {
    if dir.is_dir() {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, paths);
            } else {
                paths.push(path);
            }
        }
    }
}
