// vsql_currency — ISO 4217 currency-code type for VillageSQL.
// Rust port of adjust/pg-currency (https://github.com/adjust/pg-currency).
// SPDX-License-Identifier: GPL-2.0-only

use std::cmp::Ordering;
use villagesql::{InValue, VdfReturn};

/// The supported ISO 4217 currency codes, in ascending alphabetical order.
///
/// A value of the `currency` type is stored as a single byte: the index of
/// its code in this table. Because the table is sorted, that index is also
/// the sort key, so byte-order comparison yields alphabetical order.
const CODES: [&str; 164] = [
    "AED", "AFN", "ALL", "AMD", "ANG", "AOA", "ARS", "AUD",
    "AWG", "AZN", "BAM", "BBD", "BDT", "BGN", "BHD", "BIF",
    "BMD", "BND", "BOB", "BRL", "BSD", "BTN", "BWP", "BYN",
    "BYR", "BZD", "CAD", "CDF", "CHF", "CLP", "CNY", "COP",
    "CRC", "CUC", "CUP", "CVE", "CZK", "DJF", "DKK", "DOP",
    "DZD", "EGP", "ERN", "ETB", "EUR", "FJD", "FKP", "GBP",
    "GEL", "GGP", "GHS", "GIP", "GMD", "GNF", "GTQ", "GYD",
    "HKD", "HNL", "HRK", "HTG", "HUF", "IDR", "ILS", "IMP",
    "INR", "IQD", "IRR", "ISK", "JEP", "JMD", "JOD", "JPY",
    "KES", "KGS", "KHR", "KMF", "KPW", "KRW", "KWD", "KYD",
    "KZT", "LAK", "LBP", "LKR", "LRD", "LSL", "LTL", "LYD",
    "MAD", "MDL", "MGA", "MKD", "MMK", "MNT", "MOP", "MRO",
    "MUR", "MVR", "MWK", "MXN", "MYR", "MZN", "NAD", "NGN",
    "NIO", "NOK", "NPR", "NZD", "OMR", "PAB", "PEN", "PGK",
    "PHP", "PKR", "PLN", "PYG", "QAR", "RON", "RSD", "RUB",
    "RWF", "SAR", "SBD", "SCR", "SDG", "SEK", "SGD", "SHP",
    "SLL", "SOS", "SPL", "SRD", "STD", "SVC", "SYP", "SZL",
    "THB", "TJS", "TMT", "TND", "TOP", "TRY", "TTD", "TVD",
    "TWD", "TZS", "UAH", "UGX", "USD", "UYU", "UZS", "VEF",
    "VND", "VUV", "WST", "XAF", "XCD", "XDR", "XOF", "XPF",
    "YER", "ZAR", "ZMW", "ZWD",
];

/// A `currency` is stored as one byte, so the code table cannot exceed 256
/// entries; this guards the `index as u8` narrowing in `code_index` should the
/// table ever grow.
const _: () = assert!(CODES.len() <= 256);

/// Resolve a textual code to its stored byte index.
///
/// Input is case-insensitive (`'usd'` and `'USD'` are equivalent), surrounding
/// whitespace is ignored, and the code must be exactly three ASCII letters
/// naming a supported ISO 4217 code. Returns `None` for anything else.
fn code_index(code: &str) -> Option<u8> {
    let code = code.trim();
    if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    // Uppercase the three ASCII letters in a stack buffer — no heap allocation
    // on this per-row path. The bytes are ASCII, so the buffer is valid UTF-8.
    let mut upper = [0u8; 3];
    for (slot, byte) in upper.iter_mut().zip(code.bytes()) {
        *slot = byte.to_ascii_uppercase();
    }
    let upper = std::str::from_utf8(&upper).ok()?;
    CODES.binary_search(&upper).ok().map(|index| index as u8)
}

/// The stored byte of a value. `persisted_length` is 1, so the server always
/// supplies exactly one byte; a missing byte falls back to index 0 for the
/// total functions (compare, hash) whose signatures cannot report an error.
fn stored_index(stored: &[u8]) -> u8 {
    stored.first().copied().unwrap_or(0)
}

/// Read the leading argument of a VDF as a trimmed string. `Ok(None)` means a
/// SQL NULL (or no argument); `Err` carries a message for a non-string input.
fn string_arg<'a>(args: &'a [InValue], func: &str) -> Result<Option<&'a str>, String> {
    match args.first() {
        Some(InValue::String(s)) => Ok(Some(s.trim())),
        Some(InValue::Null) | None => Ok(None),
        _ => Err(format!("{func}: expected a STRING argument")),
    }
}

/// Parse a currency code into its single stored byte. Unknown or malformed
/// codes are rejected so an invalid value can never be stored.
fn currency_encode(s: &str) -> Result<Vec<u8>, String> {
    code_index(s)
        .map(|index| vec![index])
        .ok_or_else(|| format!("currency: '{s}' is not a supported ISO 4217 code"))
}

/// Render a stored byte back into its uppercase ISO 4217 code.
fn currency_decode(stored: &[u8]) -> Result<String, String> {
    match stored.first() {
        Some(&index) if (index as usize) < CODES.len() => Ok(CODES[index as usize].to_owned()),
        _ => Err("currency: stored value is out of range".to_owned()),
    }
}

/// Order two stored currencies alphabetically by their code.
///
/// The stored byte is the index into the alphabetically sorted code table, so
/// comparing the bytes directly reproduces alphabetical ordering.
fn currency_compare(a: &[u8], b: &[u8]) -> Ordering {
    stored_index(a).cmp(&stored_index(b))
}

/// Hash a stored currency. The single stored byte uniquely identifies the
/// code, so it is the hash.
fn currency_hash(stored: &[u8]) -> usize {
    stored_index(stored) as usize
}

/// Return the supported ISO 4217 codes whose name begins with `prefix`, as a
/// JSON array of strings in alphabetical order. The prefix is case-insensitive.
///
/// VEF has no set-returning functions and a scalar string result is capped at
/// 256 bytes — too small for all 164 codes at once. Enumeration is therefore
/// chunked by prefix: pass a single letter (`'U'`) to list that group, or a
/// longer prefix to narrow further. Callers unpack the array into rows with
/// `JSON_TABLE()`; wrap the call in `CONVERT(... USING utf8mb4)` for the JSON
/// functions. Returns NULL for a NULL argument; errors on an empty prefix.
fn supported_currencies(args: &[InValue]) -> VdfReturn {
    let prefix = match string_arg(args, "supported_currencies") {
        Ok(Some(prefix)) => prefix,
        Ok(None) => return VdfReturn::null(),
        Err(message) => return VdfReturn::error(message),
    };
    if prefix.is_empty() {
        return VdfReturn::error(format!(
            "supported_currencies: a non-empty prefix is required \
             (the full {}-code list exceeds the 256-byte return limit); \
             query by first letter, e.g. supported_currencies('U')",
            CODES.len()
        ));
    }
    if !prefix.bytes().all(|b| b.is_ascii_alphabetic()) {
        return VdfReturn::error("supported_currencies: prefix must be ASCII letters");
    }
    let upper = prefix.to_ascii_uppercase();
    let mut json = String::with_capacity(128);
    json.push('[');
    let mut first = true;
    for code in CODES.iter().filter(|code| code.starts_with(&upper)) {
        if !first {
            json.push(',');
        }
        first = false;
        json.push('"');
        json.push_str(code);
        json.push('"');
    }
    json.push(']');
    VdfReturn::string(json)
}

/// Return the total number of supported ISO 4217 codes.
fn currency_count(_args: &[InValue]) -> VdfReturn {
    VdfReturn::int(CODES.len() as i64)
}

/// Return 1 if `code` is a supported ISO 4217 code, 0 otherwise. The check is
/// case-insensitive. Returns NULL for a NULL argument.
fn is_currency(args: &[InValue]) -> VdfReturn {
    match string_arg(args, "is_currency") {
        Ok(Some(code)) => VdfReturn::int(i64::from(code_index(code).is_some())),
        Ok(None) => VdfReturn::null(),
        Err(message) => VdfReturn::error(message),
    }
}

villagesql::extension! {
    funcs: [
        villagesql::func!(
            supported_currencies,
            "supported_currencies",
            [villagesql::Type::String] -> villagesql::Type::String,
            deterministic: true
        ),
        villagesql::func!(
            currency_count,
            "currency_count",
            [] -> villagesql::Type::Int,
            deterministic: true
        ),
        villagesql::func!(
            is_currency,
            "is_currency",
            [villagesql::Type::String] -> villagesql::Type::Int,
            deterministic: true
        ),
    ],
    types: [
        villagesql::custom_type!(
            type_name: "currency",
            persisted_length: 1,
            max_decode_buffer_length: 3,
            encode: currency_encode,
            decode: currency_decode,
            compare: currency_compare,
            hash: currency_hash,
            default: "AED",
        ),
    ]
}
