# VillageSQL Currency Extension

`vsql_currency` adds a compact `CURRENCY` data type to VillageSQL that stores an
ISO 4217 currency code in a single byte. It is a Rust port of
[adjust/pg-currency](https://github.com/adjust/pg-currency) and supports the same
164 currency codes, with case-insensitive input, alphabetical ordering, and
indexing.

```sql
INSTALL EXTENSION vsql_currency;

CREATE TABLE accounts (id INT PRIMARY KEY, balance_currency vsql_currency.currency);
INSERT INTO accounts VALUES (1, 'USD'), (2, 'eur');
SELECT id, balance_currency FROM accounts ORDER BY balance_currency;
-- 2  EUR
-- 1  USD
```

## Building

Requires the [VillageSQL Rust SDK](https://github.com/villagesql/vsql-rust-sdk)
(the `villagesql` crate, pulled from crates.io) and the `cargo-vsql` CLI
(`cargo install cargo-vsql`). A stable Rust toolchain 1.87 or newer is required.

```bash
export VillageSQL_BUILD_DIR=/path/to/villagesql/build
cargo vsql install
```

`cargo vsql install` compiles in release mode, packages the `.veb` archive, and
copies it into the server's VEB output directory.

## Installing

```sql
INSTALL EXTENSION vsql_currency;
```

To uninstall, first drop or migrate any columns that use the type (the server
will not uninstall while a `currency` column exists), then:

```sql
UNINSTALL EXTENSION vsql_currency;
```

## The CURRENCY type

`vsql_currency.currency` stores a three-letter ISO 4217 code in one byte (an
index into the supported-code table).

- **Input** is case-insensitive and must be exactly three letters naming a
  supported code: `'USD'`, `'usd'`, and `'  eur '` are all accepted; surrounding
  whitespace is ignored. Unknown codes, wrong-length input, and non-letter input
  are rejected, so an invalid value can never be stored.
- **Output** is always the uppercase canonical code (`USD`, `EUR`, ...).
- **Ordering and equality** follow alphabetical order of the code, so `ORDER BY`,
  `=`, `<`, `>`, `BETWEEN`, `GROUP BY`, and `DISTINCT` all work directly on a
  `currency` column.
- **Indexing** is supported: a `currency` column can carry a `KEY`/`UNIQUE` index
  and be looked up through it.

```sql
CREATE TABLE price (id INT PRIMARY KEY AUTO_INCREMENT, c vsql_currency.currency, KEY (c));
INSERT INTO price (c) VALUES ('USD'), ('jpy'), ('EUR');
SELECT c FROM price WHERE c = 'JPY';     -- JPY
SELECT c FROM price ORDER BY c;          -- EUR, JPY, USD
```

### Reading and casting

Custom types are not wired into MySQL's `CAST(... AS ...)` grammar, so there is no
`CAST(x AS currency)`. To obtain a `currency` value, insert a string literal into a
`currency` column (the column type drives the conversion). Selecting the column
returns the three-letter code as text.

### Default value

The type carries an internal default of `AED` (the SDK requires every custom type
to have an encodable default). This default is **not** exposed as a SQL column
default: a nullable column stores `NULL` when no value is given, and a `NOT NULL`
column with no value and no explicit `DEFAULT` raises an error as usual. `AED` is
never silently stored on your behalf.

## Function Reference

### `vsql_currency.currency_count() -> INT`

Returns the number of supported ISO 4217 codes (`164`). Takes no arguments.

```sql
SELECT vsql_currency.currency_count();   -- 164
```

### `vsql_currency.is_currency(code VARCHAR) -> INT`

Returns `1` if `code` is a supported ISO 4217 code, `0` otherwise. The check is
case-insensitive and ignores surrounding whitespace. Returns `NULL` for a `NULL`
argument (standard MySQL NULL propagation).

```sql
SELECT vsql_currency.is_currency('USD');   -- 1
SELECT vsql_currency.is_currency('usd');   -- 1
SELECT vsql_currency.is_currency('XYZ');   -- 0
SELECT vsql_currency.is_currency(NULL);    -- NULL
```

### `vsql_currency.supported_currencies(prefix VARCHAR) -> JSON array (string)`

Returns the supported codes whose name begins with `prefix` (case-insensitive),
as a JSON array of strings in alphabetical order. Returns `NULL` for a `NULL`
argument. An empty prefix raises an error, and a non-letter prefix raises an
error.

A single scalar string result is limited to 256 bytes, which is too small for all
164 codes at once, so enumeration is chunked by prefix — pass a first letter to
list that group, or a longer prefix to narrow further.

The result carries the `binary` character set, so wrap it in
`CONVERT(... USING utf8mb4)` before handing it to MySQL's JSON functions. Use
`JSON_TABLE()` to expand the array into rows:

```sql
SELECT code
FROM JSON_TABLE(
  CONVERT(vsql_currency.supported_currencies('U') USING utf8mb4),
  '$[*]' COLUMNS (code CHAR(3) PATH '$')
) AS j;
-- UAH, UGX, USD, UYU, UZS

-- List everything by iterating the alphabet (one call per letter):
SELECT j.code
FROM (SELECT 'A' AS p UNION SELECT 'B' UNION SELECT 'C' /* ... through 'Z' */) AS letters
JOIN JSON_TABLE(
  CONVERT(vsql_currency.supported_currencies(letters.p) USING utf8mb4),
  '$[*]' COLUMNS (code CHAR(3) PATH '$')
) AS j
ORDER BY j.code;
```

### Generated type functions

Registering the type also generates `vsql_currency.currency::from_string`,
`::to_string`, `::compare`, and `::hash`. These back the type's storage,
comparison, and indexing; you normally interact with the type through ordinary
SQL (column storage, `=`, `ORDER BY`) rather than calling them directly.

## Migrating from PostgreSQL

### Function and capability mapping

| pg-currency (PostgreSQL) | vsql_currency (VillageSQL) |
|---|---|
| `currency` type, input `'USD'` | `vsql_currency.currency`, input `'USD'` (case-insensitive) |
| `currency_in` / `currency_out` | implicit on column store / `SELECT` |
| comparison operators `=,<,>,<=,>=` | same operators, directly on the column |
| B-tree index on a `currency` column | `KEY`/`UNIQUE` index on a `currency` column |
| `supported_currencies()` (returns a row set) | `vsql_currency.supported_currencies(prefix)` (returns a JSON array; expand with `JSON_TABLE`) |
| — | `vsql_currency.currency_count()` (added) |
| — | `vsql_currency.is_currency(code)` (added) |

### Before / after

```sql
-- PostgreSQL: list all supported currencies
SELECT * FROM supported_currencies() ORDER BY currency;

-- VillageSQL: enumerate by prefix and expand with JSON_TABLE
SELECT code FROM JSON_TABLE(
  CONVERT(vsql_currency.supported_currencies('U') USING utf8mb4),
  '$[*]' COLUMNS (code CHAR(3) PATH '$')
) AS j;
```

```sql
-- PostgreSQL and VillageSQL are identical here:
CREATE TABLE t (c currency);                 -- PG
CREATE TABLE t (c vsql_currency.currency);   -- VillageSQL
INSERT INTO t VALUES ('USD'), ('eur');
SELECT c FROM t ORDER BY c;                   -- EUR, USD (both)
```

### Behavioral differences

- **Case-insensitive input** and **alphabetical ordering** match pg-currency.
- **NULL handling** follows MySQL: a NULL argument to a function yields NULL.
- **Set-returning function:** `supported_currencies()` cannot return a row set
  (VEF has no SRFs) and a scalar string is capped at 256 bytes, so it returns a
  JSON array chunked by prefix; expand with `JSON_TABLE`.
- **JSON character set:** function string results are `binary`; wrap them in
  `CONVERT(... USING utf8mb4)` for JSON functions.
- **Aggregates:** `MIN`, `MAX`, `COUNT(*)`, `COUNT(DISTINCT c)`, and
  `GROUP_CONCAT(c)` work on a `currency` column. `SUM`, `AVG`, and `COUNT(c)` are
  rejected (a currency code is not a number).
- **No `CAST(... AS currency)`** — store via a column instead.
- **Upgrades** require `UNINSTALL` + `INSTALL` (there is no `ALTER EXTENSION`), and
  dependent columns must be dropped or migrated first.

### Blocked functions and their workaround

- `supported_currencies()` as a true set-returning function — **blocked**; the
  prefix-based JSON array above is the supported workaround.

## Known Limitations

Each limitation links the upstream VillageSQL issue whose resolution would remove
the workaround. If a limitation affects you, give the issue a 👍 to signal demand.

- **No set-returning function; 256-byte scalar string cap.** The full 164-code
  list cannot be returned in one call. `supported_currencies(prefix)` enumerates
  by prefix; `currency_count()` and `is_currency()` cover counting and membership.
  A native set-returning / table-function API
  ([villagesql-server#549](https://github.com/villagesql/villagesql-server/issues/549))
  and a larger / dynamic VDF string return
  ([#641](https://github.com/villagesql/villagesql-server/issues/641),
  [#343](https://github.com/villagesql/villagesql-server/issues/343)) would remove
  the prefix-chunking workaround.
- **Function string results use the `binary` character set.** Wrap calls in
  `CONVERT(... USING utf8mb4)` before passing them to JSON functions. Tracked by
  [#612](https://github.com/villagesql/villagesql-server/issues/612) (string
  returns need charset metadata).
- **No `SUM`/`AVG`/`COUNT(column)` over a `currency` column.** Use `COUNT(*)`,
  `COUNT(DISTINCT)`, `MIN`, `MAX`, or `GROUP_CONCAT`. Opt-in numeric promotion for
  custom types is tracked by
  [#605](https://github.com/villagesql/villagesql-server/issues/605).
- **No in-place upgrade.** Changing the extension requires `UNINSTALL` +
  `INSTALL`; drop or migrate dependent columns first. The upgrade story is tracked
  by [#344](https://github.com/villagesql/villagesql-server/issues/344).

## Security Considerations

The extension performs no I/O, network access, or filesystem access, and holds no
secrets. All input is strictly validated against a fixed table of codes before it
is stored, and the JSON produced by `supported_currencies` contains only fixed,
quote-free ASCII codes from that table — never user-supplied content — so there is
no JSON-injection surface.

## Testing

See [TESTING.md](TESTING.md).

## Contributing

See the [VillageSQL Contributing Guide](https://github.com/villagesql/villagesql-server/blob/main/CONTRIBUTING.md).

## Reporting Bugs and Requesting Features

Please open an issue at
[github.com/villagesql/vsql-currency/issues](https://github.com/villagesql/vsql-currency/issues).

## Contact

- Discord: https://discord.gg/KSr6whd3Fr
- GitHub Issues: https://github.com/villagesql/vsql-currency/issues

## License

GPL-2.0. See [LICENSE](LICENSE).
