# First-hour adopter walkthrough

**Deferred by owner decision, 2026-10-12.** No participant recruitment is required
to continue development. Retain this protocol for a future voluntary session;
do not present an agent rehearsal as human evidence.

This is the human evidence protocol for roadmap item 6. A developer new to the
current Kujo workflow should attempt it on a fresh user account or disposable
machine, using published instructions. An agent rehearsal is preparation only.
The observer records friction and help given, rather than completing tasks for
the participant. Stop at 60 minutes and record incomplete tasks honestly.

## Before starting

Record participant alias (no personal details required), OS/architecture, shell,
prior Kujo experience, existing Node/Rust availability and whether Kujo/Kennel are
already installed. Record the exact documentation URLs and revision/date read.
Choose macOS or Linux for the complete current Kennel path; native Windows Kujo
support does not imply a native Windows Kennel installer.

Start a timer before opening the instructions. Record elapsed time when the
participant first runs their own working program; do not substitute installation
or agent execution time. Keep command output locally, excluding credentials and
unrelated environment values. A command's failure is evidence, not a reason to
silently switch channels or edit the recorded result.

## Tasks

1. Follow [Installation](../INSTALLATION.md) to install Kujo. Record the chosen
   channel, exact version and resolved executable path. Confirm the version is
   1.8.2. Record prerequisite/PATH confusion and any help used.
2. Use the [README quickstart](../README.md#run-your-first-program) to create,
   check and run `hello.kujo` on the default VM. Change the greeting and run it
   again. Record time to this first working program.
3. Follow [Install Kennel](../INSTALLATION.md#install-kennel), then use the
   documented `kennel init --name my-project`, `kennel add ability`, and
   `kennel install` flow in a new directory. Inspect `kennel.toml` and
   `kennel.lock`; explain how these differ from Kujo's built-in manifests.
   Record the exact client/package versions, outputs and any trust prompt.
4. Create `broken.kujo` containing `print(missing_name)`. Run it, explain the
   diagnostic, repair it to print a string, and run it again. Record the error,
   exit status, attempted repair and whether the message provided enough help.
5. Create `message.txt` with harmless text and `read.kujo` containing
   `print(read_file("message.txt"))`. Using the
   [safety instructions](../README.md#safety-model-snapshot), first demonstrate
   denial with `--untrusted`, then grant only file-read permission and succeed.
   Explain why these capability flags do not make Kujo an OS sandbox.

## Record, do not infer

Copy this form into a dated evidence file after the walkthrough. Leave unknown
values empty; a pending form is not proof of success.

| Field | Observed value |
| --- | --- |
| Participant alias / prior experience | |
| OS / architecture / shell | |
| Fresh account or machine / preinstalled tools | |
| Instructions URL + revision/date | |
| Install channel / runtime path / version | |
| Kennel client / package versions | |
| Started at / first own program at / elapsed minutes | |
| Task 1 install: pass/fail/incomplete + evidence | |
| Task 2 program: pass/fail/incomplete + evidence | |
| Task 3 packages: pass/fail/incomplete + evidence | |
| Task 4 diagnosis: pass/fail/incomplete + evidence | |
| Task 5 capabilities: pass/fail/incomplete + evidence | |
| Help received / misleading wording / attempted fixes | |
| Clean-install logs and artifact checksums | |
| Local fixes and participant retest result | |
| External-owner follow-ups | |

If this optional study is resumed, completing it requires the actual human record, clean-install evidence, fixes and
retests for confirmed blockers, and agreement between public guidance and the
shipped runtime. Report agent observations separately in
[the preparation audit](audits/first-hour-adopter-validation.md).
