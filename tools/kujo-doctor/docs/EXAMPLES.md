# Kujo Doctor Examples

## Generic checks

```bash
kujo doctor
```

## JSON output

```bash
kujo doctor --json
```

## Deep mode

```bash
kujo doctor --deep
```

## Profile discovery

```bash
kujo doctor --list-profiles
```

## Profile execution

When profile contributions are installed/registered:

```bash
kujo doctor wordpress
kujo doctor --profile vercel
```

## Canonical workflow-pack path

```bash
kujo pack run doctor doctor
kujo pack run doctor doctor --json
```
