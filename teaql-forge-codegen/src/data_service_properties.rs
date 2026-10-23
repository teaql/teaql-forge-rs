//! DataServiceNaming properties shared by native generation and external preview.
use crate::context::RenderDataService;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use teaql_forge_model::data_services::DataService;

pub(crate) fn rust_runtime(
    service_name: &str,
    attributes: BTreeMap<String, String>,
) -> RenderDataService {
    let sqlite_connection = "    let url = &config.database_url;\n    let sanitized_url = if url.starts_with(\"sqlite:\") { url.strip_prefix(\"sqlite:\").unwrap().trim_start_matches(\"//\") } else { url };\n    let pure_file_path = sanitized_url.split('?').next().unwrap_or(sanitized_url);\n    let path = std::path::Path::new(pure_file_path);\n    if let Some(parent) = path.parent() { if !parent.as_os_str().is_empty() { std::fs::create_dir_all(parent).map_err(|e| ServiceRuntimeError::ConnectionError(e.to_string()))?; } }\n    Ok(std::sync::Arc::new(std::sync::Mutex::new(rusqlite::Connection::open(pure_file_path).map_err(|e| ServiceRuntimeError::ConnectionError(e.to_string()))?)))";
    let (provider, driver, ext, dialect, executor, error, id_generator, pool, use_method, connect) =
        match service_name {
            "postgres" | "pg" => (
                "teaql-provider-postgres",
                "tokio-postgres = { version = \"0.7\" }\ndeadpool-postgres = { version = \"0.14.0\", features = [\"rt_tokio_1\"] }\ndeadpool = \"0.12\"\ntokio-util = \"0.7\"\ndeadpool-runtime = \"0.1.4\"",
                "teaql_provider_postgres::PostgresProviderExt",
                "teaql_provider_postgres::PostgresDialect",
                "teaql_provider_postgres::PgMutationExecutor",
                "teaql_provider_postgres::MutationExecutorError",
                "teaql_provider_postgres::PgIdSpaceGenerator",
                "deadpool_postgres::Pool",
                "use_postgres_provider",
                "    let mut pg_config = config.database_url.parse::<tokio_postgres::Config>().map_err(|e| ServiceRuntimeError::ConnectionError(e.to_string()))?;\n    pg_config.user(&config.database_user).password(&config.database_password);\n    let mgr = deadpool_postgres::Manager::new(pg_config, tokio_postgres::NoTls);\n    let pool = deadpool_postgres::Pool::builder(mgr).build().map_err(|e| ServiceRuntimeError::ConnectionError(e.to_string()))?;\n    Ok(pool)",
            ),
            "mysql" => (
                "teaql-provider-mysql",
                "mysql_async = { version = \"0.37\", default-features = false, features = [\"default-rustls\", \"chrono\"] }",
                "teaql_provider_mysql::MysqlProviderExt",
                "teaql_provider_mysql::MysqlDialect",
                "teaql_provider_mysql::MysqlMutationExecutor",
                "teaql_provider_mysql::MutationExecutorError",
                "teaql_provider_mysql::MysqlIdSpaceGenerator",
                "mysql_async::Pool",
                "use_mysql_provider",
                "    let opts = mysql_async::Opts::from_url(&config.database_url).map_err(|e| ServiceRuntimeError::ConnectionError(e.to_string()))?;\n    let opts = mysql_async::OptsBuilder::from_opts(opts)\n        .user(Some(config.database_user.clone()))\n        .pass(Some(config.database_password.clone()));\n    Ok(mysql_async::Pool::new(opts))",
            ),
            "rusqlite" => (
                "teaql-provider-sqlite",
                "rusqlite = { version = \"0.40\", features = [\"bundled\", \"chrono\", \"column_decltype\"] }",
                "teaql_provider_sqlite::RusqliteProviderExt",
                "teaql_provider_sqlite::RusqliteDialect",
                "teaql_provider_sqlite::RusqliteMutationExecutor",
                "teaql_provider_sqlite::MutationExecutorError",
                "teaql_provider_sqlite::RusqliteIdSpaceGenerator",
                "std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>",
                "use_rusqlite_provider",
                sqlite_connection,
            ),
            "sqlite" => (
                "teaql-provider-sqlite",
                "rusqlite = { version = \"0.40\", features = [\"bundled\", \"chrono\", \"column_decltype\"] }",
                "teaql_provider_sqlite::SqliteProviderExt",
                "teaql_provider_sqlite::SqliteDialect",
                "teaql_provider_sqlite::SqliteMutationExecutor",
                "teaql_provider_sqlite::MutationExecutorError",
                "teaql_provider_sqlite::SqliteIdSpaceGenerator",
                "std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>",
                "use_sqlite_provider",
                sqlite_connection,
            ),
            _ => ("", "", "", "", "", "", "", "", "", ""),
        };
    RenderDataService {
        name: service_name.to_string(),
        attributes,
        rust_has_sql_provider: !provider.is_empty(),
        rust_sqlx_dependency: driver.to_string(),
        rust_sql_provider_ext_trait: ext.to_string(),
        rust_sql_dialect_type: dialect.to_string(),
        rust_sql_mutation_executor_type: executor.to_string(),
        rust_sql_mutation_error_type: error.to_string(),
        rust_sql_id_generator_type: id_generator.to_string(),
        rust_sql_pool_type: pool.to_string(),
        rust_sql_use_provider_method: use_method.to_string(),
        rust_sql_connect_pool_code: connect.to_string(),
        rust_sql_supports_transactions: matches!(service_name, "sqlite" | "rusqlite"),
        rust_sql_needs_credentials: !matches!(service_name, "sqlite" | "rusqlite"),
    }
}

pub(crate) fn rust_provider_crate(name: &str) -> Option<&'static str> {
    match name {
        "postgres" | "pg" => Some("teaql-provider-postgres"),
        "mysql" => Some("teaql-provider-mysql"),
        "sqlite" | "rusqlite" => Some("teaql-provider-sqlite"),
        _ => None,
    }
}

pub(crate) fn public_name(name: &str, attributes: &BTreeMap<String, String>) -> Value {
    match attributes.get("name").filter(|value| !value.is_empty()) {
        Some(value) => match value.to_ascii_lowercase().as_str() {
            "true" | "on" | "yes" => json!(true),
            "false" | "off" | "no" => json!(false),
            _ => json!(value),
        },
        None => json!(name),
    }
}

pub(crate) fn preview_value(service: &DataService) -> Value {
    let name = service.name.as_str();
    let attributes = service
        .attributes
        .iter()
        .map(|attr| (attr.name.clone(), attr.value.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut data = Map::new();
    data.insert(
        "__stg_java_class".into(),
        json!("com.skynet.codegenerator.meta.DataService"),
    );
    data.insert("name".into(), json!(name));
    data.insert("default".into(), json!(service.is_default));
    data.insert("supportStack".into(), json!(service.support_stack));
    for (key, value) in &attributes {
        if value.is_empty() {
            continue;
        }
        data.insert(
            key.clone(),
            match value.to_ascii_lowercase().as_str() {
                "true" | "on" | "yes" => json!(true),
                "false" | "off" | "no" => json!(false),
                _ => json!(value),
            },
        );
    }
    data.insert("attributes".into(), json!(service.attributes));
    data.insert("__stg_attributes".into(), json!(attributes));
    data.insert("__stg_service_name".into(), json!(name));
    let mut getters =
        crate::preview_beans::inherited_getters(Some(name), &json!(attributes), 0, None);
    getters["default"] = json!(service.is_default);
    data.insert("__stg_java_getters".into(), getters);

    // Enhancers precede attached attributes; their switches use the bean name,
    // even when an attached `name` changes the public preview property.
    let postgres = matches!(name, "postgres" | "pg");
    let mysql = name == "mysql";
    let sqlite = name == "sqlite";
    let sqlserver = matches!(name, "mssql" | "sqlserver");
    let sql_provider = postgres || mysql || sqlite;
    let executor = match name {
        "sqlite" => "io.teaql.core.sqlite.SqliteDataServiceExecutor",
        "postgres" | "pg" => "io.teaql.core.postgres.PostgresDataServiceExecutor",
        "mysql" => "io.teaql.core.mysql.MysqlDataServiceExecutor",
        "oracle" => "io.teaql.core.oracle.OracleDataServiceExecutor",
        "mssql" | "sqlserver" => "io.teaql.core.mssql.MssqlDataServiceExecutor",
        "duck" | "duckdb" => "io.teaql.core.duck.DuckDataServiceExecutor",
        _ => "io.teaql.dataservice.sql.SqlDataServiceExecutor",
    };
    data.insert("javaDataServiceExecutorClass".into(), json!(executor));
    data.insert(
        "javaDataServiceExecutorNeedsDataSource".into(),
        json!(sqlite || mysql),
    );
    data.insert(
        "javaCredentialsRequired".into(),
        json!(!matches!(name, "sqlite" | "duck" | "duckdb")),
    );
    data.insert("javaQuarkusManualSqliteDataSource".into(), json!(sqlite));
    data.insert("javaQuarkusUsesAgroal".into(), json!(!sqlite));
    data.insert(
        "javaQuarkusJdbcExtensionArtifact".into(),
        json!(match name {
            "postgres" | "pg" => Some("quarkus-jdbc-postgresql"),
            "mysql" => Some("quarkus-jdbc-mysql"),
            "oracle" => Some("quarkus-jdbc-oracle"),
            "mssql" | "sqlserver" => Some("quarkus-jdbc-mssql"),
            "db2" => Some("quarkus-jdbc-db2"),
            _ => None,
        }),
    );
    data.insert(
        "javaQuarkusDatabaseKind".into(),
        json!(match name {
            "postgres" | "pg" => "postgresql",
            "mysql" => "mysql",
            "oracle" => "oracle",
            "mssql" | "sqlserver" => "mssql",
            "db2" => "db2",
            _ => "other",
        }),
    );
    for language in ["typescript", "python", "golang", "dotnet"] {
        data.insert(format!("{language}IsPostgres"), json!(postgres));
        data.insert(format!("{language}IsMysql"), json!(mysql));
        data.insert(format!("{language}IsSqlite"), json!(sqlite));
        if language != "golang" {
            data.insert(
                format!("{language}HasSqlProvider"),
                json!(sql_provider || (language == "dotnet" && sqlserver)),
            );
        }
    }
    data.insert("dotnetIsSqlServer".into(), json!(sqlserver));
    for (key, pg, my, other) in [
        (
            "golangSqlDriverImport",
            "github.com/lib/pq",
            "github.com/go-sql-driver/mysql",
            "github.com/mattn/go-sqlite3",
        ),
        (
            "golangSqlDriverDependency",
            "github.com/lib/pq v1.10.9",
            "github.com/go-sql-driver/mysql v1.7.1",
            "github.com/mattn/go-sqlite3 v1.14.16",
        ),
        ("golangSqlDriverName", "postgres", "mysql", "sqlite3"),
        (
            "golangSqlProviderImport",
            "github.com/teaql/teaql-golang/provider/postgres",
            "github.com/teaql/teaql-golang/provider/mysql",
            "github.com/teaql/teaql-golang/provider/sqlite",
        ),
        (
            "golangSqlDialectType",
            "PostgresDialect",
            "MysqlDialect",
            "SqliteDialect",
        ),
        (
            "golangSqlMutationExecutorConstructor",
            "NewPgMutationExecutor",
            "NewMysqlMutationExecutor",
            "NewSqliteMutationExecutor",
        ),
    ] {
        data.insert(
            key.into(),
            json!(if postgres {
                pg
            } else if mysql {
                my
            } else {
                other
            }),
        );
    }
    let driver = |key: &str| {
        service
            .attribute(key)
            .filter(|value| !value.chars().all(crate::input_path::blank_char))
    };
    let dependency = driver("driverGroupId")
        .zip(driver("driverArtifactId"))
        .map(|(group, artifact)| format!("{group}:{artifact}"));
    data.insert("driverDependency".into(), json!(dependency));
    for key in [
        "driverGroupId",
        "driverArtifactId",
        "driverVersion",
        "driverClass",
    ] {
        data.insert(key.into(), json!(driver(key)));
    }

    let runtime = rust_runtime(name, attributes);
    let rust_provider = rust_provider_crate(name);
    let rust_data_provider = rust_provider.is_some() || name == "linux";
    data.insert("rustHasSqlDialect".into(), json!(rust_provider.is_some()));
    data.insert("rustHasSqlProvider".into(), json!(rust_provider.is_some()));
    data.insert("rustSupportsBusinessIdInfrastructure".into(), json!(sqlite));
    data.insert(
        "rustBusinessIdAllocatorType".into(),
        json!(sqlite.then_some("teaql_provider_sqlite::SqliteBusinessIdAllocator")),
    );
    data.insert("rustIsSqlx".into(), json!(name != "rusqlite"));
    data.insert("rustIsLinuxProvider".into(), json!(name == "linux"));
    data.insert(
        "rustHasDataServiceProvider".into(),
        json!(rust_data_provider),
    );
    data.insert("rustNeedsTokioRuntime".into(), json!(rust_data_provider));
    data.insert(
        "rustDataServiceExecutorType".into(),
        json!(if name == "linux" {
            "teaql_provider_linux::LinuxDataServiceExecutor"
        } else {
            "crate::runtime::DataServiceExecutor"
        }),
    );
    data.insert(
        "rustProviderCrate".into(),
        json!(if name == "linux" {
            Some("teaql-provider-linux")
        } else {
            rust_provider
        }),
    );
    data.insert("rustSqlProviderCrate".into(), json!(rust_provider));
    let dependency = rust_provider.map(|name| format!("{name} = \"5.0.5\""));
    data.insert("rustSqlProviderDependency".into(), json!(dependency));
    data.insert("rustSqlDialectDependency".into(), json!(dependency));
    for (key, value) in [
        ("rustSqlDialectType", runtime.rust_sql_dialect_type),
        (
            "rustSqlMutationExecutorType",
            runtime.rust_sql_mutation_executor_type,
        ),
        (
            "rustSqlMutationErrorType",
            runtime.rust_sql_mutation_error_type,
        ),
        ("rustSqlIdGeneratorType", runtime.rust_sql_id_generator_type),
        (
            "rustSqlProviderExtTrait",
            runtime.rust_sql_provider_ext_trait,
        ),
        (
            "rustSqlUseProviderMethod",
            runtime.rust_sql_use_provider_method,
        ),
        ("rustSqlPoolType", runtime.rust_sql_pool_type),
        ("rustSqlConnectPoolCode", runtime.rust_sql_connect_pool_code),
        ("rustSqlxDependency", runtime.rust_sqlx_dependency),
    ] {
        data.insert(key.into(), json!((!value.is_empty()).then_some(value)));
    }
    data.insert("rustSqlConnectOptionsType".into(), json!("\"\""));
    data.insert(
        "rustSqlNeedsCredentials".into(),
        json!(runtime.rust_sql_needs_credentials),
    );
    data.insert(
        "rustSqlSupportsTransactions".into(),
        json!(runtime.rust_sql_supports_transactions),
    );
    Value::Object(data)
}
