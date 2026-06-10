# Kujo Doctor Security

## Security posture

- No network access required by default.
- No file writes are performed by doctor checks.
- Process probes are routed through Kujo's safe workflow process runner.
- Output capture is truncated to avoid unbounded log dumping.

## Process probing policy

Doctor checks may invoke local developer tooling commands such as:

- `git`
- `node`
- `npm`
- `php`
- `composer`
- `wp` (optional)

## Secret handling

- Doctor does not intentionally read or print secrets.
- Command output is sampled/truncated; avoid embedding sensitive data in tool banners/hooks.

## Responsible disclosure

If a dedicated security contact is not yet published for Kujo Doctor, report issues through the main Kujo repository security process.
