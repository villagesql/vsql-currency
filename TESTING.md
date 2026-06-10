# Testing vsql_currency

The test suite uses the standard MySQL test runner (MTR), driven by `cargo-vsql`.

## Requirements

- A stable Rust toolchain 1.87 or newer.
- `cargo-vsql` (`cargo install cargo-vsql`).
- A built VillageSQL server. Point `cargo-vsql` at its build directory:

```bash
export VillageSQL_BUILD_DIR=/path/to/villagesql/build
```

## Build and install

```bash
cargo vsql install
```

This compiles the extension in release mode, packages the `.veb`, and copies it
into the server's VEB output directory so the suite can `INSTALL EXTENSION` it.

## Run the suite

```bash
cargo vsql test
```

Each test file installs the extension, creates an isolated `currency_db`
database, exercises behavior, then drops the database and uninstalls the
extension, leaving no residual state.

## Regenerate expected output

After intentionally changing assertions, regenerate the `.result` files from
actual output:

```bash
cargo vsql test --record
```

Never edit `.result` files by hand — always record them.

## Test files

Test inputs live in `mysql-test/t/` and expected output in `mysql-test/r/`.

| Test file | Covers |
|---|---|
| `currency_type.test` | Storing and reading back a code, case-insensitive input, NULL handling, and rejection of unknown / wrong-length / non-letter codes. |
| `currency_compare.test` | Alphabetical ordering, equality and range comparison, indexing, and which aggregate functions are available (`COUNT(*)`, `COUNT(DISTINCT)`, `MIN`, `MAX`, `GROUP_CONCAT` work; `SUM`, `AVG`, `COUNT(column)` are rejected). |
| `currency_catalog.test` | `currency_count()`, `is_currency()` (including case-insensitivity and NULL), and `supported_currencies(prefix)` with `JSON_TABLE` expansion, the `CONVERT(... USING utf8mb4)` requirement, and the empty-prefix error. |
