# Standard Library Inventory

Status: v1.0.0 baseline draft (active)
Last updated: 2026-06-09

This inventory is the canonical support table for runtime-native functions registered by `Interpreter::get_builtin_names()` in `src/interpreter/mod.rs`.

Arity key:

- `exact N`: strict arity, exactly N arguments
- `A..=B`: inclusive range arity
- `variadic (N+)`: at least N arguments
- `handler-defined`: arity/type checks are currently enforced in the native handler implementation instead of centralized `native_callable_arity` metadata

Capability key:

- `none`: no capability gate
- other values map to `NativeCapability::as_str()` and require explicit allow flags in restricted mode

JSON conversion contract (`parse_json` / `to_json` / `to_json_pretty`):

- `parse_json` enforces a maximum input size of `1,048,576` bytes and a maximum nesting depth of `64`.
- Invalid JSON returns a `Value::Error` message including parse-location details from `serde_json`.
- `to_json` and `to_json_pretty` reject non-finite floats (`NaN`, `+/-inf`) with a `Value::Error` instead of silently coercing values.
- Dictionary-like values are serialized with deterministic key ordering (lexicographic for string keys, ascending for integer keys).

| Function | Signature | Arity | Return Type | Errors | Capability | Example |
| --- | --- | --- | --- | --- | --- | --- |
| `print` | `print(...)` | variadic (0+) | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := print(...)` |
| `eprint` | `eprint(...)` | variadic (0+) | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := eprint(...)` |
| `println` | `println(...)` | variadic (0+) | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := println(...)` |
| `__vm_for_iterable` | `__vm_for_iterable(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := __vm_for_iterable(...)` |
| `abs` | `abs(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := abs(...)` |
| `sqrt` | `sqrt(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := sqrt(...)` |
| `pow` | `pow(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := pow(...)` |
| `floor` | `floor(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := floor(...)` |
| `ceil` | `ceil(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := ceil(...)` |
| `round` | `round(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := round(...)` |
| `min` | `min(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := min(...)` |
| `max` | `max(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := max(...)` |
| `sin` | `sin(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := sin(...)` |
| `cos` | `cos(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := cos(...)` |
| `tan` | `tan(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := tan(...)` |
| `log` | `log(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := log(...)` |
| `exp` | `exp(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := exp(...)` |
| `bit_and` | `bit_and(left, right)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bit_and(...)` |
| `bit_or` | `bit_or(left, right)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bit_or(...)` |
| `bit_xor` | `bit_xor(left, right)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bit_xor(...)` |
| `bit_not` | `bit_not(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bit_not(...)` |
| `bit_shl` | `bit_shl(left, right)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bit_shl(...)` |
| `bit_shr` | `bit_shr(left, right)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bit_shr(...)` |
| `len` | `len(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := len(...)` |
| `substring` | `substring(value, start, end)` | exact 3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := substring(...)` |
| `substr` | `substr(value, start, end)` | exact 3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := substr(...)` |
| `to_upper` | `to_upper(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_upper(...)` |
| `upper` | `upper(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := upper(...)` |
| `to_lower` | `to_lower(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_lower(...)` |
| `lower` | `lower(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := lower(...)` |
| `capitalize` | `capitalize(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := capitalize(...)` |
| `trim` | `trim(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := trim(...)` |
| `trim_start` | `trim_start(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := trim_start(...)` |
| `trim_end` | `trim_end(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := trim_end(...)` |
| `contains` | `contains(value, needle)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := contains(...)` |
| `replace_str` | `replace_str(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := replace_str(...)` |
| `replace` | `replace(value, from, to)` | exact 3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := replace(...)` |
| `split` | `split(value, delimiter)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := split(...)` |
| `join` | `join(values, separator)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := join(...)` |
| `ssg_render_pages` | `ssg_render_pages(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := ssg_render_pages(...)` |
| `ssg_build_output_paths` | `ssg_build_output_paths(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := ssg_build_output_paths(...)` |
| `ssg_render_and_write_pages` | `ssg_render_and_write_pages(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := ssg_render_and_write_pages(...)` |
| `ssg_read_render_and_write_pages` | `ssg_read_render_and_write_pages(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := ssg_read_render_and_write_pages(...)` |
| `starts_with` | `starts_with(value, prefix)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := starts_with(...)` |
| `ends_with` | `ends_with(value, suffix)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := ends_with(...)` |
| `pad_left` | `pad_left(value, width, pad_char)` | exact 3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := pad_left(...)` |
| `pad_right` | `pad_right(value, width, pad_char)` | exact 3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := pad_right(...)` |
| `pad_start` | `pad_start(value, width, pad_char)` | exact 3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := pad_start(...)` |
| `pad_end` | `pad_end(value, width, pad_char)` | exact 3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := pad_end(...)` |
| `lines` | `lines(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := lines(...)` |
| `words` | `words(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := words(...)` |
| `str_reverse` | `str_reverse(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := str_reverse(...)` |
| `slugify` | `slugify(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := slugify(...)` |
| `truncate` | `truncate(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := truncate(...)` |
| `to_camel_case` | `to_camel_case(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_camel_case(...)` |
| `to_snake_case` | `to_snake_case(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_snake_case(...)` |
| `to_kebab_case` | `to_kebab_case(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_kebab_case(...)` |
| `index_of` | `index_of(value, needle)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := index_of(...)` |
| `repeat` | `repeat(value, count)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := repeat(...)` |
| `char_at` | `char_at(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := char_at(...)` |
| `is_empty` | `is_empty(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_empty(...)` |
| `count_chars` | `count_chars(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := count_chars(...)` |
| `push` | `push(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := push(...)` |
| `append` | `append(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := append(...)` |
| `pop` | `pop(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := pop(...)` |
| `insert` | `insert(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := insert(...)` |
| `remove` | `remove(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := remove(...)` |
| `remove_at` | `remove_at(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := remove_at(...)` |
| `clear` | `clear(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := clear(...)` |
| `slice` | `slice(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := slice(...)` |
| `concat` | `concat(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := concat(...)` |
| `map` | `map(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := map(...)` |
| `filter` | `filter(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := filter(...)` |
| `reduce` | `reduce(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := reduce(...)` |
| `find` | `find(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := find(...)` |
| `sort` | `sort(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := sort(...)` |
| `reverse` | `reverse(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := reverse(...)` |
| `unique` | `unique(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := unique(...)` |
| `sum` | `sum(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := sum(...)` |
| `any` | `any(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := any(...)` |
| `all` | `all(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := all(...)` |
| `chunk` | `chunk(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := chunk(...)` |
| `flatten` | `flatten(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := flatten(...)` |
| `zip` | `zip(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := zip(...)` |
| `enumerate` | `enumerate(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := enumerate(...)` |
| `take` | `take(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := take(...)` |
| `skip` | `skip(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := skip(...)` |
| `windows` | `windows(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := windows(...)` |
| `range` | `range(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := range(...)` |
| `format` | `format(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := format(...)` |
| `keys` | `keys(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := keys(...)` |
| `values` | `values(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := values(...)` |
| `items` | `items(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := items(...)` |
| `has_key` | `has_key(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := has_key(...)` |
| `get` | `get(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := get(...)` |
| `merge` | `merge(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := merge(...)` |
| `invert` | `invert(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := invert(...)` |
| `update` | `update(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := update(...)` |
| `get_default` | `get_default(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := get_default(...)` |
| `input` | `input(prompt?)` | 0..=1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := input(...)` |
| `parse_int` | `parse_int(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := parse_int(...)` |
| `parse_float` | `parse_float(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := parse_float(...)` |
| `to_int` | `to_int(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_int(...)` |
| `to_float` | `to_float(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_float(...)` |
| `to_string` | `to_string(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_string(...)` |
| `str` | `str(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := str(...)` |
| `to_bool` | `to_bool(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_bool(...)` |
| `bytes` | `bytes(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bytes(...)` |
| `dict` | `dict()` | exact 0 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := dict(...)` |
| `array` | `array(...)` | variadic (0+) | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := array(...)` |
| `error` | `error(message)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := error(...)` |
| `type` | `type(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := type(...)` |
| `type_of` | `type_of(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := type_of(...)` |
| `is_truthy` | `is_truthy(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_truthy(...)` |
| `is_int` | `is_int(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_int(...)` |
| `is_float` | `is_float(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_float(...)` |
| `is_string` | `is_string(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_string(...)` |
| `is_bool` | `is_bool(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_bool(...)` |
| `is_array` | `is_array(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_array(...)` |
| `is_dict` | `is_dict(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_dict(...)` |
| `is_null` | `is_null(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_null(...)` |
| `is_function` | `is_function(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_function(...)` |
| `assert` | `assert(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := assert(...)` |
| `debug` | `debug(...)` | variadic (0+) | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := debug(...)` |
| `read_file` | `read_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := read_file(...)` |
| `read_file_lossy` | `read_file_lossy(path)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := read_file_lossy(...)` |
| `write_file` | `write_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := write_file(...)` |
| `append_file` | `append_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := append_file(...)` |
| `file_exists` | `file_exists(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := file_exists(...)` |
| `read_lines` | `read_lines(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := read_lines(...)` |
| `list_dir` | `list_dir(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := list_dir(...)` |
| `create_dir` | `create_dir(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := create_dir(...)` |
| `file_size` | `file_size(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := file_size(...)` |
| `delete_file` | `delete_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-delete` | `result := delete_file(...)` |
| `rename_file` | `rename_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := rename_file(...)` |
| `copy_file` | `copy_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := copy_file(...)` |
| `read_binary_file` | `read_binary_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := read_binary_file(...)` |
| `write_binary_file` | `write_binary_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := write_binary_file(...)` |
| `io_read_bytes` | `io_read_bytes(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := io_read_bytes(...)` |
| `io_write_bytes` | `io_write_bytes(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := io_write_bytes(...)` |
| `io_append_bytes` | `io_append_bytes(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := io_append_bytes(...)` |
| `io_read_at` | `io_read_at(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := io_read_at(...)` |
| `io_write_at` | `io_write_at(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := io_write_at(...)` |
| `io_seek_read` | `io_seek_read(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := io_seek_read(...)` |
| `io_file_metadata` | `io_file_metadata(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := io_file_metadata(...)` |
| `io_truncate` | `io_truncate(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := io_truncate(...)` |
| `io_copy_range` | `io_copy_range(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := io_copy_range(...)` |
| `parse_json` | `parse_json(json_string)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation, oversized input (>1,048,576 bytes), excessive nesting (>64), invalid JSON parse, or capability-denied when gated. | `none` | `result := parse_json("{\"ok\":true}")` |
| `to_json` | `to_json(value)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation, unsupported value conversion, non-finite float serialization, or capability-denied when gated. | `none` | `result := to_json({"ok": true})` |
| `to_json_pretty` | `to_json_pretty(value)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation, unsupported value conversion, non-finite float serialization, or capability-denied when gated. | `none` | `result := to_json_pretty({"ok": true})` |
| `parse_toml` | `parse_toml(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := parse_toml(...)` |
| `to_toml` | `to_toml(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_toml(...)` |
| `parse_yaml` | `parse_yaml(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := parse_yaml(...)` |
| `to_yaml` | `to_yaml(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_yaml(...)` |
| `parse_csv` | `parse_csv(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := parse_csv(...)` |
| `to_csv` | `to_csv(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_csv(...)` |
| `encode_base64` | `encode_base64(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := encode_base64(...)` |
| `decode_base64` | `decode_base64(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := decode_base64(...)` |
| `random` | `random(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := random(...)` |
| `random_int` | `random_int(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := random_int(...)` |
| `random_choice` | `random_choice(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := random_choice(...)` |
| `uuid_v4` | `uuid_v4(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := uuid_v4(...)` |
| `random_id` | `random_id(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := random_id(...)` |
| `set_random_seed` | `set_random_seed(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := set_random_seed(...)` |
| `clear_random_seed` | `clear_random_seed(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := clear_random_seed(...)` |
| `now` | `now(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := now(...)` |
| `now_utc` | `now_utc(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := now_utc(...)` |
| `now_unix` | `now_unix(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := now_unix(...)` |
| `now_utc_seconds` | `now_utc_seconds(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := now_utc_seconds(...)` |
| `current_timestamp` | `current_timestamp(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := current_timestamp(...)` |
| `time` | `time(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := time(...)` |
| `performance_now` | `performance_now(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := performance_now(...)` |
| `time_us` | `time_us(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := time_us(...)` |
| `time_ns` | `time_ns(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := time_ns(...)` |
| `format_duration` | `format_duration(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := format_duration(...)` |
| `elapsed` | `elapsed(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := elapsed(...)` |
| `format_date` | `format_date(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := format_date(...)` |
| `parse_date` | `parse_date(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := parse_date(...)` |
| `env` | `env(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `env-read` | `result := env(...)` |
| `env_or` | `env_or(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `env-read` | `result := env_or(...)` |
| `env_int` | `env_int(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `env-read` | `result := env_int(...)` |
| `env_float` | `env_float(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `env-read` | `result := env_float(...)` |
| `env_bool` | `env_bool(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `env-read` | `result := env_bool(...)` |
| `env_required` | `env_required(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `env-read` | `result := env_required(...)` |
| `kv_set` | `kv_set(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := kv_set(...)` |
| `kv_get` | `kv_get(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := kv_get(...)` |
| `env_set` | `env_set(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `env-write` | `result := env_set(...)` |
| `env_list` | `env_list(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `env-read` | `result := env_list(...)` |
| `args` | `args(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := args(...)` |
| `arg_parser` | `arg_parser(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := arg_parser(...)` |
| `exit` | `exit(code?)` | 0..=1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := exit(...)` |
| `sleep` | `sleep(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := sleep(...)` |
| `execute` | `execute(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `shell-exec` | `result := execute(...)` |
| `execute_status` | `execute_status(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `shell-exec` | `result := execute_status(...)` |
| `os_getcwd` | `os_getcwd(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := os_getcwd(...)` |
| `os_chdir` | `os_chdir(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := os_chdir(...)` |
| `os_rmdir` | `os_rmdir(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-delete` | `result := os_rmdir(...)` |
| `os_environ` | `os_environ(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := os_environ(...)` |
| `join_path` | `join_path(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := join_path(...)` |
| `dirname` | `dirname(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := dirname(...)` |
| `basename` | `basename(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := basename(...)` |
| `path_exists` | `path_exists(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_exists(...)` |
| `path_join` | `path_join(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_join(...)` |
| `path_absolute` | `path_absolute(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_absolute(...)` |
| `path_is_dir` | `path_is_dir(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_is_dir(...)` |
| `path_is_file` | `path_is_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_is_file(...)` |
| `path_is_symlink` | `path_is_symlink(...)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_is_symlink(...)` |
| `path_extension` | `path_extension(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_extension(...)` |
| `regex_match` | `regex_match(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := regex_match(...)` |
| `regex_find_all` | `regex_find_all(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := regex_find_all(...)` |
| `regex_replace` | `regex_replace(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := regex_replace(...)` |
| `regex_split` | `regex_split(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := regex_split(...)` |
| `http_get` | `http_get(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_get(...)` |
| `http_request` | `http_request(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_request(...)` |
| `http_post` | `http_post(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_post(...)` |
| `http_put` | `http_put(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_put(...)` |
| `http_delete` | `http_delete(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_delete(...)` |
| `http_get_binary` | `http_get_binary(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_get_binary(...)` |
| `ai_chat` | `ai_chat(prompt_or_messages, options)` | exact 2 | dynamic (Value) | Value::Error on invalid args/options contracts; `Result(Err)` with deterministic transport/API response failures. | `network-client` | `result := ai_chat("Hi", {"endpoint":"https://example.ai/chat","model":"gpt"})` |
| `ai_stream_chat` | `ai_stream_chat(prompt_or_messages, options)` | exact 2 | dynamic (Value) | Value::Error on invalid args/options contracts; `Result(Err)` with deterministic transport/API response failures. | `network-client` | `result := ai_stream_chat("Hi", {"endpoint":"https://example.ai/chat","model":"gpt"})` |
| `ai_embedding` | `ai_embedding(input, options)` | exact 2 | dynamic (Value) | Value::Error on invalid args/options contracts; `Result(Err)` if embedding response contract is missing `data[0].embedding`. | `network-client` | `result := ai_embedding("query", {"endpoint":"https://example.ai/embed","model":"text-embed"})` |
| `ai_tool_loop` | `ai_tool_loop(prompt_or_messages, options)` | exact 2 | dynamic (Value) | Value::Error on invalid args/options contracts; `Result(Err)` for missing tool results or deterministic transport/API failures. | `network-client` | `result := ai_tool_loop("Plan this", {"endpoint":"https://example.ai/chat","model":"gpt","tool_results":{"lookup":"ok"}})` |
| `parallel_http` | `parallel_http(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := parallel_http(...)` |
| `jwt_encode` | `jwt_encode(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := jwt_encode(...)` |
| `jwt_decode` | `jwt_decode(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := jwt_decode(...)` |
| `jwt_verify` | `jwt_verify(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := jwt_verify(...)` |
| `oauth2_auth_url` | `oauth2_auth_url(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := oauth2_auth_url(...)` |
| `oauth2_get_token` | `oauth2_get_token(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := oauth2_get_token(...)` |
| `http_get_stream` | `http_get_stream(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_get_stream(...)` |
| `http_server` | `http_server(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := http_server(...)` |
| `http_listen` | `http_listen(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-server` | `result := http_listen(...)` |
| `http_response` | `http_response(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := http_response(...)` |
| `json_response` | `json_response(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := json_response(...)` |
| `html_response` | `html_response(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := html_response(...)` |
| `redirect_response` | `redirect_response(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := redirect_response(...)` |
| `set_header` | `set_header(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_header(...)` |
| `set_headers` | `set_headers(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_headers(...)` |
| `db_connect` | `db_connect(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_connect(...)` |
| `db_execute` | `db_execute(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_execute(...)` |
| `db_query` | `db_query(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_query(...)` |
| `db_close` | `db_close(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_close(...)` |
| `db_pool` | `db_pool(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_pool(...)` |
| `db_pool_acquire` | `db_pool_acquire(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_pool_acquire(...)` |
| `db_pool_release` | `db_pool_release(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_pool_release(...)` |
| `db_pool_stats` | `db_pool_stats(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_pool_stats(...)` |
| `db_pool_close` | `db_pool_close(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_pool_close(...)` |
| `db_begin` | `db_begin(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_begin(...)` |
| `db_commit` | `db_commit(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_commit(...)` |
| `db_rollback` | `db_rollback(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_rollback(...)` |
| `db_last_insert_id` | `db_last_insert_id(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_last_insert_id(...)` |
| `Set` | `Set(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := Set(...)` |
| `set_add` | `set_add(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_add(...)` |
| `set_has` | `set_has(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_has(...)` |
| `set_remove` | `set_remove(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_remove(...)` |
| `set_union` | `set_union(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_union(...)` |
| `set_intersect` | `set_intersect(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_intersect(...)` |
| `set_difference` | `set_difference(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_difference(...)` |
| `set_to_array` | `set_to_array(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_to_array(...)` |
| `Queue` | `Queue(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := Queue(...)` |
| `queue_enqueue` | `queue_enqueue(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := queue_enqueue(...)` |
| `queue_dequeue` | `queue_dequeue(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := queue_dequeue(...)` |
| `queue_peek` | `queue_peek(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := queue_peek(...)` |
| `queue_size` | `queue_size(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := queue_size(...)` |
| `queue_is_empty` | `queue_is_empty(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := queue_is_empty(...)` |
| `queue_to_array` | `queue_to_array(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := queue_to_array(...)` |
| `Stack` | `Stack(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := Stack(...)` |
| `stack_push` | `stack_push(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := stack_push(...)` |
| `stack_pop` | `stack_pop(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := stack_pop(...)` |
| `stack_peek` | `stack_peek(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := stack_peek(...)` |
| `stack_size` | `stack_size(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := stack_size(...)` |
| `stack_is_empty` | `stack_is_empty(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := stack_is_empty(...)` |
| `stack_to_array` | `stack_to_array(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := stack_to_array(...)` |
| `channel` | `channel(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := channel(...)` |
| `shared_set` | `shared_set(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := shared_set(...)` |
| `shared_get` | `shared_get(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := shared_get(...)` |
| `shared_has` | `shared_has(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := shared_has(...)` |
| `shared_delete` | `shared_delete(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := shared_delete(...)` |
| `shared_add_int` | `shared_add_int(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := shared_add_int(...)` |
| `async_sleep` | `async_sleep(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := async_sleep(...)` |
| `async_timeout` | `async_timeout(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := async_timeout(...)` |
| `async_http_get` | `async_http_get(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := async_http_get(...)` |
| `async_http_post` | `async_http_post(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := async_http_post(...)` |
| `async_read_file` | `async_read_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := async_read_file(...)` |
| `async_read_files` | `async_read_files(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := async_read_files(...)` |
| `async_write_file` | `async_write_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := async_write_file(...)` |
| `async_write_files` | `async_write_files(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := async_write_files(...)` |
| `spawn_task` | `spawn_task(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := spawn_task(...)` |
| `await_task` | `await_task(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := await_task(...)` |
| `cancel_task` | `cancel_task(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := cancel_task(...)` |
| `Promise.all` | `Promise.all(promises, concurrency?)` | 1..=2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := Promise.all(...)` |
| `promise_all` | `promise_all(promises, concurrency?)` | 1..=2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := promise_all(...)` |
| `await_all` | `await_all(promises, concurrency?)` | 1..=2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := await_all(...)` |
| `parallel_map` | `parallel_map(items, mapper, concurrency?)` | 2..=3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := parallel_map(...)` |
| `par_map` | `par_map(items, mapper, concurrency?)` | 2..=3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := par_map(...)` |
| `par_each` | `par_each(items, mapper, concurrency?)` | 2..=3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := par_each(...)` |
| `set_task_pool_size` | `set_task_pool_size(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_task_pool_size(...)` |
| `get_task_pool_size` | `get_task_pool_size(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := get_task_pool_size(...)` |
| `assert_equal` | `assert_equal(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := assert_equal(...)` |
| `assert_true` | `assert_true(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := assert_true(...)` |
| `assert_false` | `assert_false(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := assert_false(...)` |
| `assert_contains` | `assert_contains(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := assert_contains(...)` |
| `load_image` | `load_image(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := load_image(...)` |
| `gif_to_webp` | `gif_to_webp(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := gif_to_webp(...)` |
| `zip_create` | `zip_create(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := zip_create(...)` |
| `zip_add_file` | `zip_add_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := zip_add_file(...)` |
| `zip_add_dir` | `zip_add_dir(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := zip_add_dir(...)` |
| `zip_close` | `zip_close(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := zip_close(...)` |
| `unzip` | `unzip(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := unzip(...)` |
| `sha256` | `sha256(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := sha256(...)` |
| `sha256_file` | `sha256_file(path)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := sha256_file(...)` |
| `md5` | `md5(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := md5(...)` |
| `md5_file` | `md5_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := md5_file(...)` |
| `hash_password` | `hash_password(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := hash_password(...)` |
| `verify_password` | `verify_password(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := verify_password(...)` |
| `aes_encrypt` | `aes_encrypt(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := aes_encrypt(...)` |
| `aes_decrypt` | `aes_decrypt(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := aes_decrypt(...)` |
| `aes_encrypt_bytes` | `aes_encrypt_bytes(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := aes_encrypt_bytes(...)` |
| `aes_decrypt_bytes` | `aes_decrypt_bytes(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := aes_decrypt_bytes(...)` |
| `rsa_generate_keypair` | `rsa_generate_keypair(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := rsa_generate_keypair(...)` |
| `rsa_encrypt` | `rsa_encrypt(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := rsa_encrypt(...)` |
| `rsa_decrypt` | `rsa_decrypt(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := rsa_decrypt(...)` |
| `rsa_sign` | `rsa_sign(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := rsa_sign(...)` |
| `rsa_verify` | `rsa_verify(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := rsa_verify(...)` |
| `spawn_process` | `spawn_process(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `process-exec` | `result := spawn_process(...)` |
| `pipe_commands` | `pipe_commands(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `process-exec` | `result := pipe_commands(...)` |
| `tcp_listen` | `tcp_listen(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-server` | `result := tcp_listen(...)` |
| `tcp_accept` | `tcp_accept(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-server` | `result := tcp_accept(...)` |
| `tcp_connect` | `tcp_connect(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := tcp_connect(...)` |
| `tcp_send` | `tcp_send(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := tcp_send(...)` |
| `tcp_receive` | `tcp_receive(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := tcp_receive(...)` |
| `tcp_close` | `tcp_close(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := tcp_close(...)` |
| `tcp_set_nonblocking` | `tcp_set_nonblocking(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := tcp_set_nonblocking(...)` |
| `udp_bind` | `udp_bind(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-server` | `result := udp_bind(...)` |
| `udp_send_to` | `udp_send_to(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := udp_send_to(...)` |
| `udp_receive_from` | `udp_receive_from(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := udp_receive_from(...)` |
| `udp_close` | `udp_close(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := udp_close(...)` |

## Coverage Contract

The integration contract test `tests/stdlib_reference_contract.rs` verifies:

- every runtime builtin is documented here exactly once
- documented capability values match runtime capability policy mapping
- documented arity labels match centralized arity metadata for builtins that use it

When adding/removing/renaming native builtins, update runtime registration and regenerate/update this table in the same change.
