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
| `currency_type.test` | Storing and reading back a code, case-insensitive input, and NULL handling. |
| `currency_invalid_code.test` | Rejection of unknown / wrong-length / non-letter codes with the standard bad-value error (1366). Currently skipped; see below. |
| `currency_compare.test` | Alphabetical ordering, equality and range comparison, indexing, and which aggregate functions are available (`COUNT(*)`, `COUNT(DISTINCT)`, `MIN`, `MAX`, `GROUP_CONCAT` work; `SUM`, `AVG`, `COUNT(column)` are rejected). |
| `currency_catalog.test` | `currency_count()`, `is_currency()` (including case-insensitivity and NULL), and `supported_currencies(prefix)` with `JSON_TABLE` expansion both with and without `CONVERT(... USING utf8mb4)`, plus the full list an empty prefix returns. |

## Skipped tests

`currency_invalid_code.test` is skipped until the `villagesql` crate dependency
is updated past 0.0.6. In 0.0.6 an `Err` from the type's `encode` is reported
as a hard error, so a current server rejects an invalid code with 3219 and the
extension's message instead of 1366. The SDK fix reports it as a bad value.
When bumping the dependency, delete the `--skip` line (and its TODO) from the
test and run the suite.
