# First-party CLI parser package

The first-party `modules/cli.kujo` module is the HLP-004 parser package. A
checkout uses it automatically from its project `modules/` directory. An
installed Kujo executable can expose the same package to another repository by
setting `KUJO_MODULE_PATH` to a directory containing `cli.kujo`, then importing
it with `from cli import parse`.

parse(argv, spec) is pure and returns an envelope:

    success: {"ok": true, "value": {"command", "positionals", "options", "occurrences"}}
    failure: {"ok": false, "error": {"index", "token", "code", "message"}}

Each spec entry supplies name and kind ("bool" or "value"). Optional fields
are short, required, default, and allow_empty. Long flags, short aliases,
--name=value, separated values, repeated occurrences, and the -- terminator
are supported. Values remain strings so the caller owns typed conversion and
domain validation.

The package does not read process arguments, environment variables, or files;
it does not render help or exit the process. Callers can pass args() when they
want process arguments:

    from cli import parse

    spec := [{"name": "json", "kind": "bool", "short": "-j"}]
    parsed := parse(args(), spec)
    if parsed["ok"] {
        print(parsed["value"]["options"]["json"])
    }

The compatibility fixture tests/cli_module.kujo runs on both the VM default
and the explicit interpreter path.
