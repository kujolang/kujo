# How to Start an Agent Session

This guide shows you how to effectively start a conversation with an AI coding agent for working on the Kujo language.

---

## 🎯 Starting a New Session

### Step 1: Point Agent to Instructions

Always start with this prompt:

```
Read and follow the instructions in .github/AGENT_INSTRUCTIONS.md

Then read ROADMAP.md and tell me what the current high priority items are.
```

This ensures the agent:
- Understands the project standards
- Reviews the current priorities
- Knows the documentation requirements

---

## 📋 Template Prompts for Common Tasks

### Working on Next Roadmap Feature

```
Read .github/AGENT_INSTRUCTIONS.md and follow all rules.

Review ROADMAP.md and implement the next high priority feature. 

Requirements:
1. Create a todo list with all implementation steps
2. Implement the feature completely
3. Add comprehensive tests
4. Update CHANGELOG.md, ROADMAP.md, and README.md
5. Commit after each major step (not all at once)
6. Use clear, descriptive commit messages

Start with the highest priority incomplete item.
```

### Fixing Bugs

```
Read .github/AGENT_INSTRUCTIONS.md and follow all rules.

I've found a bug: [describe the bug]

Requirements:
1. Investigate and identify root cause
2. Fix the issue completely
3. Add tests to prevent regression
4. Update CHANGELOG.md under "Fixed" section
5. Commit with message: "fix: [brief description]"
6. Zero warnings required
```

### Adding Tests

```
Read .github/AGENT_INSTRUCTIONS.md and follow all rules.

Add comprehensive tests for [feature/area].

Requirements:
1. Add at least 5-10 integration tests
2. Cover happy path, edge cases, and error cases
3. All tests must pass
4. Commit with message: "test: add tests for [feature]"
```

### Preparing for Release

```
Read .github/AGENT_INSTRUCTIONS.md and follow all rules.

Prepare for [X.Y.Z] release:

1. Review CHANGELOG.md - ensure all changes documented
2. Update ROADMAP.md - mark completed items, update progress table
3. Update README.md - reflect new capabilities
4. Run full test suite and verify zero warnings
5. Update Cargo.toml version to X.Y.Z
6. Commit: ":rocket: RELEASE: X.Y.Z"
7. Create git tag: git tag -a X.Y.Z -m "Release X.Y.Z"
8. Push: git push origin main && git push origin X.Y.Z
9. Create GitHub release with CHANGELOG content

Note: Tag format is X.Y.Z (e.g., 0.2.0) NOT vX.Y.Z
```

---

## 🔄 Git Commit Workflow

### When to Commit

Commit after each of these milestones:
- ✅ Individual feature implementation
- ✅ Bug fix completed and tested
- ✅ Test suite added
- ✅ Documentation updated
- ✅ Refactoring completed

**DON'T** wait until everything is done - commit incrementally!

### Commit Message Format

Use emoji-prefixed commit messages:

```bash
git commit -m ":package: NEW: <description>"
git commit -m ":ok_hand: IMPROVE: <description>"
git commit -m ":bug: BUG: <description>"
git commit -m ":book: DOC: <description>"
```

**Types:**
- `:package: NEW:` - New feature
- `:bug: BUG:` - Bug fix
- `:ok_hand: IMPROVE:` - Improvements, tests, refactoring
- `:book: DOC:` - Documentation only
- `:rocket: RELEASE:` - Version releases

**Examples:**

```bash
# Feature commits
git commit -m ":package: NEW: add lexical scoping support"
git commit -m ":package: NEW: implement user input() function"

# Bug fix commits
git commit -m ":bug: BUG: field assignment now works with nested access"
git commit -m ":bug: BUG: boolean conditions properly evaluate truthy values"

# Test/improvement commits
git commit -m ":ok_hand: IMPROVE: add integration tests for field assignment"
git commit -m ":ok_hand: IMPROVE: add scoping tests for nested blocks"
git commit -m ":ok_hand: IMPROVE: eliminate compiler warnings"

# Documentation commits
git commit -m ":book: DOC: update CHANGELOG for v0.2.0"
git commit -m ":book: DOC: update ROADMAP to remove completed features"

# Release commits
git commit -m ":rocket: RELEASE: 0.2.0"
git commit -m ":rocket: RELEASE: 0.3.0"
```

---

## 📝 Example Full Session

Here's a complete example of working on a roadmap item:

```
Read .github/AGENT_INSTRUCTIONS.md and follow all rules.

Implement the "Lexical Scoping" feature from ROADMAP.md.

This is the #1 high priority item for v0.3.0.

Requirements:
1. Read the feature description in ROADMAP.md
2. Create a detailed todo list for implementation
3. Implement environment stack with parent scope lookup
4. Add comprehensive tests (at least 8-10 tests)
5. Update CHANGELOG.md with "Added" entry
6. Update ROADMAP.md progress table
7. Update README.md if syntax changes

Git workflow:
- Commit after implementing core scoping logic
- Commit after adding tests
- Commit after updating docs

Each commit should be atomic and have clear message like:
- ":package: NEW: implement lexical scoping with environment stack"
- ":ok_hand: IMPROVE: add integration tests for nested scopes"
- ":book: DOC: document lexical scoping in CHANGELOG"

Zero warnings required. All tests must pass.
```

---

## ⚠️ What NOT to Do

❌ **Vague requests:**
```
Work on the next thing
```

❌ **Skipping instructions:**
```
Implement lexical scoping
```
*(Agent won't know the standards)*

❌ **One giant commit:**
```
Commit everything at the end
```

❌ **No documentation requirements:**
```
Just implement the feature
```

---

## ✅ Best Practices

1. **Always reference agent instructions** - Ensures consistency
2. **Be specific about requirements** - Include tests, docs, commits
3. **Request todo lists** - Keeps work organized and visible
4. **Require incremental commits** - Creates clear history
5. **Enforce zero warnings** - Maintains code quality
6. **Verify before finishing** - Ask agent to run tests and examples

---

## 🎯 Quick Start Commands

### Most Common: Work on Next Feature

```
Read .github/AGENT_INSTRUCTIONS.md and follow all rules.
Implement the next high priority feature from ROADMAP.md.
Create todo list first, then implement with tests and docs.
Commit after each major step.
```

### Bug Fixing Session

```
Read .github/AGENT_INSTRUCTIONS.md and follow all rules.
[Describe bug]
Fix completely, add tests, update docs, commit.
```

### Review Session

```
Read .github/AGENT_INSTRUCTIONS.md.
Review ROADMAP.md and tell me:
1. What's the current version
2. What's completed in this version
3. What's next for the upcoming version
4. Estimated effort for next 3 priorities
```

---

## 🔍 Verifying Agent Understood

The agent should respond with:
1. Confirmation it read the instructions
2. Summary of the task
3. A todo list or plan
4. Questions if anything is unclear

If the agent doesn't create a todo list or mention commits, remind it:
```
Please create a detailed todo list first and remember to commit after each major step.
```

---

*Following this guide ensures consistent, high-quality work on the Kujo language with proper version control and documentation.*
