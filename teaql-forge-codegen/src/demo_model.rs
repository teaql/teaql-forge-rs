//! DemoModelResolver: system property, environment, then the bundled merchant.
use std::{io::Read, path::PathBuf};

pub struct ReadError {
    pub exception_class: &'static str,
    pub message: String,
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

fn nonblank(value: &String) -> bool {
    !value.chars().all(crate::input_path::blank_char)
}

pub fn path() -> String {
    crate::process_properties::property("teaql.demo.model")
        .filter(nonblank)
        .or_else(|| std::env::var("TEAQL_DEMO_MODEL").ok().filter(nonblank))
        .unwrap_or_else(|| "classpath:models/merchant.xml".into())
}

pub fn filename() -> String {
    let value = path();
    let value = value
        .strip_prefix("classpath:")
        .unwrap_or(&value)
        .trim_end_matches('/');
    value.rsplit('/').next().unwrap_or("").into()
}

fn bundled(name: &str) -> Option<&'static [u8]> {
    Some(match name {
        // The generator and model modules expose separate classpath roots.
        // Their directory streams list the first matching root's children in
        // the pinned JDK's collation order; infra is in the second module.
        "models" | "models/" => b"bank.xml\nmerchant_simplified.xml\nmerchant.xml\ntodo.xml\n",
        "models/infra" | "models/infra/" => b"iam.xml\nsaas-service-basic.xml\nsystem-types.xml\n",
        "models/bank.xml" => include_bytes!("../resources/models/bank.xml"),
        "models/todo.xml" => include_bytes!("../resources/models/todo.xml"),
        "models/merchant.xml" => include_bytes!("../resources/models/merchant.xml"),
        "models/merchant_simplified.xml" => {
            include_bytes!("../resources/models/merchant_simplified.xml")
        }
        "models/infra/iam.xml" => include_bytes!("../resources/models/infra/iam.xml"),
        "models/infra/saas-service-basic.xml" => {
            include_bytes!("../resources/models/infra/saas-service-basic.xml")
        }
        "models/infra/system-types.xml" => {
            include_bytes!("../resources/models/infra/system-types.xml")
        }
        _ => return None,
    })
}

fn classpath() -> Vec<PathBuf> {
    std::env::var_os("CLASSPATH")
        .map(|value| {
            std::env::split_paths(&value)
                .map(|path| {
                    let path = if path.as_os_str().is_empty() {
                        PathBuf::from(".")
                    } else {
                        path
                    };
                    // The launcher turns classpath entries into canonical
                    // file URLs, including symlinked directory/JAR roots.
                    path.canonicalize().unwrap_or(path)
                })
                .collect()
        })
        .unwrap_or_default()
}

enum Resource {
    File(PathBuf),
    Archive(Vec<u8>, PathBuf),
    Bundled(&'static [u8]),
}

fn absolute_file(path: PathBuf) -> PathBuf {
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    let text = path.to_string_lossy();
    let trimmed = text.trim_end_matches('/');
    PathBuf::from(if trimmed.is_empty() { "/" } else { trimmed })
}

fn resource(name: &str) -> Option<Resource> {
    let normalized = crate::input_path::normalize(name);
    let name = normalized.strip_prefix('/').unwrap_or(&normalized);
    for entry in classpath() {
        if entry.is_dir() {
            // FileLoader first checks the normalized resource URL against its
            // base URL, then canonicalizes names containing "..". An earlier
            // classpath directory must not expose a sibling directory.
            let base = absolute_file(entry.clone());
            let path = PathBuf::from(crate::input_path::normalize(
                &absolute_file(entry.join(name)).to_string_lossy(),
            ));
            let base = PathBuf::from(crate::input_path::normalize(&base.to_string_lossy()));
            if !path.starts_with(&base) {
                continue;
            }
            if name.contains("..") {
                let (Ok(canonical), Ok(base)) = (path.canonicalize(), base.canonicalize()) else {
                    continue;
                };
                if !canonical
                    .to_string_lossy()
                    .starts_with(base.to_string_lossy().as_ref())
                {
                    continue;
                }
            }
            if path.exists() {
                return Some(Resource::File(absolute_file(path)));
            }
        } else if let Ok(file) = std::fs::File::open(&entry) {
            if let Ok(mut archive) = zip::ZipArchive::new(file) {
                // JarLoader also finds an explicit directory entry when the
                // resource name omits its final slash. It does not synthesize
                // directories merely because files have that path prefix.
                let member_name = if archive.by_name(name).is_ok() {
                    name.to_owned()
                } else {
                    format!("{name}/")
                };
                if let Ok(mut member) = archive.by_name(&member_name) {
                    let mut bytes = Vec::new();
                    if member.read_to_end(&mut bytes).is_ok() {
                        // FileUtil converts a jar URL to a filesystem path;
                        // only an explicit classpath: request opens the entry.
                        return Some(Resource::Archive(
                            bytes,
                            absolute_file(PathBuf::from(format!("{}!/{name}", entry.display()))),
                        ));
                    }
                }
            }
        }
    }
    bundled(name).map(Resource::Bundled)
}

fn io_error(message: String) -> ReadError {
    ReadError {
        exception_class: "cn.hutool.core.io.IORuntimeException",
        message,
    }
}

fn file_bytes(file: &std::path::Path) -> Result<Vec<u8>, ReadError> {
    let mut input = std::fs::File::open(file).map_err(|cause| {
        io_error(format!(
            "FileNotFoundException: {} ({})",
            file.display(),
            native_error(&cause)
        ))
    })?;
    let mut bytes = Vec::new();
    input
        .read_to_end(&mut bytes)
        .map_err(|cause| io_error(format!("IOException: {}", native_error(&cause))))?;
    Ok(bytes)
}

fn resource_bytes(resource: Resource) -> Result<Vec<u8>, ReadError> {
    match resource {
        Resource::Archive(bytes, _) => Ok(bytes),
        Resource::Bundled(bytes) => Ok(bytes.to_vec()),
        Resource::File(path) if path.is_dir() => {
            let entries = std::fs::read_dir(&path).map_err(|_| {
                io_error(format!(
                    "FileNotFoundException: {} exists, but is not accessible",
                    path.display()
                ))
            })?;
            let mut names = Vec::new();
            for entry in entries {
                let entry = entry.map_err(|_| {
                    io_error(format!(
                        "FileNotFoundException: {} exists, but is not accessible",
                        path.display()
                    ))
                })?;
                let filename = entry.file_name();
                #[cfg(unix)]
                {
                    use std::os::unix::ffi::OsStrExt;
                    names.push(crate::java_xml::utf8_string(filename.as_bytes()));
                }
                #[cfg(not(unix))]
                names.push(filename.to_string_lossy().into_owned());
            }
            crate::file_collation::sort(&mut names);
            Ok(names
                .into_iter()
                .flat_map(|name| name.into_bytes().into_iter().chain([b'\n']))
                .collect())
        }
        Resource::File(path) => file_bytes(&path),
    }
}

pub(crate) fn native_error(error: &std::io::Error) -> String {
    #[cfg(unix)]
    if let Some(code) = error.raw_os_error() {
        // HotSpot initializes native LC_MESSAGES from the process environment.
        // Use a private locale so concurrent requests never change global state.
        unsafe {
            let locale = libc::newlocale(libc::LC_ALL_MASK, c"".as_ptr(), std::ptr::null_mut());
            if !locale.is_null() {
                let previous = libc::uselocale(locale);
                let message = std::ffi::CStr::from_ptr(libc::strerror(code))
                    .to_string_lossy()
                    .into_owned();
                libc::uselocale(previous);
                libc::freelocale(locale);
                return message;
            }
        }
    }
    error.to_string()
}

pub fn read() -> Result<(Vec<u8>, String), ReadError> {
    let value = path();
    let name = filename();
    if let Some(resource_name) = value.strip_prefix("classpath:") {
        let normalized = crate::input_path::normalize(resource_name);
        let normalized = normalized.strip_prefix('/').unwrap_or(&normalized);
        let bytes = normalized.as_bytes();
        let drive = bytes.len() >= 2
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && (bytes.len() == 2 || matches!(bytes[2], b'/' | b'\\'));
        if normalized.starts_with('/') || drive {
            return Err(ReadError {
                exception_class: "java.lang.IllegalArgumentException",
                message: format!("Path [{normalized}] must be a relative path !"),
            });
        }
        return resource(resource_name)
            .ok_or_else(|| ReadError {
                exception_class: "cn.hutool.core.io.resource.NoResourceException",
                message: format!(
                    "Resource of path [{}] not exist!",
                    crate::input_path::normalize(resource_name).trim_start_matches('/')
                ),
            })
            .and_then(resource_bytes)
            .map(|bytes| (bytes, name));
    }
    let normalized = crate::input_path::normalize(&value);
    let mut file = PathBuf::from(&normalized);
    if !file.is_absolute() {
        if let Some(resource) = resource(&normalized) {
            match resource {
                Resource::File(path) | Resource::Archive(_, path) => file = path,
                Resource::Bundled(bytes) => return Ok((bytes.to_vec(), name)),
            }
        } else {
            let root = classpath()
                .into_iter()
                .find(|entry| entry.is_dir())
                .unwrap_or_else(|| PathBuf::from("."));
            file = root.join(file);
        }
    }
    file = absolute_file(file);
    if !file.exists() {
        return Err(io_error(format!("File not exist: {}", file.display())));
    }
    if !file.is_file() {
        return Err(io_error(format!("Not a file:{}", file.display())));
    }
    let bytes = file_bytes(&file)?;
    Ok((bytes, name))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::{
        ffi::OsString,
        os::unix::{
            ffi::OsStringExt,
            fs::{symlink, PermissionsExt},
        },
        process::Command,
    };

    fn unhex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn classpath_resources_match_complete_original_bytes_and_failures() {
        if let Ok(output) = std::env::var("TEAQL_RESOURCE_GOLDEN_OUTPUT") {
            let result = match read() {
                Ok((bytes, _)) => {
                    json!({"hex":bytes.iter().map(|byte|format!("{byte:02x}")).collect::<String>()})
                }
                Err(error) => {
                    json!({"exception_class":error.exception_class,"message":error.message})
                }
            };
            let mut result = result.as_object().unwrap().clone();
            result.insert("demo_path".into(), json!(path()));
            result.insert("demo_filename".into(), json!(filename()));
            std::fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
            return;
        }
        let golden: Value = serde_json::from_str(include_str!(
            "../tests/expected/classpath-resources-6f2d0a.json"
        ))
        .unwrap();
        let root =
            std::env::temp_dir().join(format!("teaql-classpath-regression-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let expand = |text: &str| text.replace("$CASE_ROOT", &root.to_string_lossy());
        for name in golden["directories"].as_object().unwrap().keys() {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        for (name, file) in golden["files"].as_object().unwrap() {
            let path = if name == "raw_filename" {
                root.join(file["parent"].as_str().unwrap())
                    .join(OsString::from_vec(unhex(
                        file["name_hex"].as_str().unwrap(),
                    )))
            } else {
                root.join(name)
            };
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, unhex(file["hex"].as_str().unwrap())).unwrap();
            std::fs::set_permissions(
                path,
                std::fs::Permissions::from_mode(file["mode"].as_u64().unwrap() as u32),
            )
            .unwrap();
        }
        for (name, target) in golden["symlinks"].as_object().unwrap() {
            symlink(expand(target.as_str().unwrap()), root.join(name)).unwrap();
        }
        for (name, mode) in golden["directories"].as_object().unwrap() {
            std::fs::set_permissions(
                root.join(name),
                std::fs::Permissions::from_mode(mode.as_u64().unwrap() as u32),
            )
            .unwrap();
        }
        let mut count = 0;
        for (group, entry) in golden["groups"].as_object().unwrap() {
            let classpath = entry["classpath"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| expand(value.as_str().unwrap()))
                .collect::<Vec<_>>()
                .join(":");
            for (value, expected) in entry["cases"].as_object().unwrap() {
                let destination = root.join("report.json");
                let mut command = Command::new(std::env::current_exe().unwrap());
                command.args(["demo_model::tests::classpath_resources_match_complete_original_bytes_and_failures","--exact","--nocapture"]);
                for name in [
                    "JAVA_TOOL_OPTIONS",
                    "JDK_JAVA_OPTIONS",
                    "_JAVA_OPTIONS",
                    "TEAQL_DEMO_MODEL",
                    "TEAQL_GENERATOR_TEMPLATE_ROOT",
                ] {
                    command.env_remove(name);
                }
                let child = command
                    .env("CLASSPATH", &classpath)
                    .env(
                        "JAVA_TOOL_OPTIONS",
                        format!("-Dteaql.demo.model=\"{}\"", expand(value)),
                    )
                    .env("TEAQL_RESOURCE_GOLDEN_OUTPUT", &destination)
                    .current_dir(&root)
                    .output()
                    .unwrap();
                assert!(
                    child.status.success(),
                    "{group}/{value}: {}",
                    String::from_utf8_lossy(&child.stderr)
                );
                let actual: Value =
                    serde_json::from_slice(&std::fs::read(&destination).unwrap()).unwrap();
                let expected: Value = serde_json::from_str(&expand(&expected.to_string())).unwrap();
                assert_eq!(actual, expected, "{group}/{value}");
                count += 1;
            }
        }
        assert_eq!(count, 175);
        for name in golden["directories"].as_object().unwrap().keys() {
            std::fs::set_permissions(root.join(name), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
