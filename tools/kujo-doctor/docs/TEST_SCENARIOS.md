# Kujo Doctor Test Scenarios

These scenarios are the baseline manual/fixture checks for Kujo Doctor behavior.

## Repository scenarios

- non-Git folder
- Git repo with clean tree
- Git repo with dirty tree

## Dependency layout scenarios

- `package.json` without `node_modules`
- `composer.json` without `vendor`
- project with no `package.json` or `composer.json`

## Tooling scenarios

- missing Node with `package.json`
- old Node with `package.json`
- unparseable version output
- WP-CLI emitting deprecation/warning output

## Build signal scenarios

- `package.json` with common scripts
