# Kujo Agent Project Implementation Report

## Executive summary

Kujo now provides a repository-owned Agent Project façade: deterministic scaffolding, discovery, inspection, fixture execution, evaluation, and the canonical Agent Doctor profile. The ownership goal is satisfied for the project contract and locally verified fixture path; live-provider and external-service behavior remains governed by the existing ecosystem components.

## Final CLI

```text
kujo agent new <name> [--profile <profile>] [--dir <path>] [--provider <id>] [--model <id>] [--no-git] [--install] [--json]
kujo agent inspect [--json]
kujo agent run [prompt] [--file <path>] [--json]
kujo agent eval [--json]
kujo doctor agent [--json] [--deep]
```

`kujo agent doctor` was not added.

## Architecture

Kujo owns the CLI façade, project discovery, capability authorization, and contract validation. Kennel owns reproducible dependency declarations. AI SDK owns provider/model selection. Agents SDK owns execution semantics and tools. Kujo Agents owns the agent package. Kujo Skills owns `SKILL.md`. MCP owns external tools; RAG owns retrieval; Dispatch owns workflows; Relay owns advanced missions; Workcell owns container-backed isolation; Watchdog owns telemetry; RunLedger owns run evidence; Eval owns general evaluation. Agent Project composes and exposes these boundaries without absorbing their internals.

## Contract and profiles

`agent.project.json` is `kujo-agent-project/v1`. Root discovery stops at the containing Git repository. Required files are canonicalized and rejected if missing or outside the root. The compositional profiles are `basic`, `tools`, `knowledge`, `workflow`, `hardened`, `observable`, and `full`.

## Security and portability

Scaffolding rejects traversal, symlink destinations, unknown profiles, and non-empty destinations. It stages the tree and atomically promotes it. Git initializes only outside an existing repository. Base scaffolding performs no network I/O. Credentials are environment-only. Generated dependencies use immutable revisions and no sibling checkout paths. Hardened configuration keeps capability authorization and Workcell isolation distinct.

## Offline and live paths

The generated fixture entrypoint runs deterministically without credentials. A live path is selected in `config/model.json` and uses an AI SDK provider plus environment credentials; live calls are not part of the offline verification claim.

## Verification record

The repository gate includes formatting, compilation, Agent CLI contract tests, all-profile scaffold checks, Doctor/inspect/run/eval fixture checks, negative path checks, and a portability copy test. Exact executed commands and commit are recorded in Git history and the final task handoff.

## Deferred external proof

Docker/Podman execution, live provider calls, remote MCP services, and a running Watchdog instance require external infrastructure and are not claimed by the local fixture verification.
