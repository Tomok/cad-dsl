# Commit Skill

You are an expert at creating high-quality commits with proper quality checks.

## Your Task

Create a git commit with the following quality guarantees:

1. **All tests must pass** - Run `cargo test`
2. **No clippy warnings** - Run `cargo clippy -- -D warnings`
3. **Code must be formatted** - Run `cargo fmt`

## Quality Check Loop

IMPORTANT: If any quality check fails (tests or clippy), you MUST:
1. Fix the issues in the code
2. Re-run ALL quality checks (tests, clippy, fmt) in sequence
3. Repeat until all checks pass

Never commit code that has failing tests or clippy warnings.

## Process

Follow these steps in order:

### 1. Format Code
```bash
cargo fmt
```

### 2. Run Clippy
```bash
cargo clippy -- -D warnings
```

If clippy reports warnings:
- Fix all warnings
- Re-run fmt
- Re-run clippy
- Continue to tests only when clippy is clean

### 3. Run Tests
```bash
cargo test
```

If tests fail:
- Fix the failing tests
- Re-run fmt
- Re-run clippy
- Re-run tests
- Repeat until all pass

### 4. Check Git Status
```bash
git status
git diff
```

Review what changes will be committed.

### 5. Create Commit Message

Analyze the changes and create a commit message following this format:

**Subject line (50 chars max):**
- Use imperative mood (e.g., "Add feature" not "Added feature")
- Start with a verb (Add, Fix, Refactor, Update, Remove, etc.)
- Be specific and concise
- No period at the end

**Body (wrap at 72 chars):**
- Explain WHY the change was made, not WHAT (the diff shows what)
- Include context and motivation
- Reference any related issues

Example:
```
Add commit skill with quality checks

Implements a custom Claude skill that ensures code quality before
committing by running tests, clippy, and formatting in a loop until
all checks pass. This prevents broken code from entering the repo.
```

### 6. Stage and Commit
```bash
git add .
git commit -m "$(cat <<'EOF'
[Your commit message here]
EOF
)"
```

### 7. Verify Commit
```bash
git log -1 --format='[%h] %s'
git status
```

## Pull Request Messages

If creating a PR, generate a PR description with:

**Title:** Same as commit subject line

**Body:**
```markdown
## Summary
- [Bullet points describing the changes]
- [Focus on user-facing impact and motivation]

## Changes Made
- [Key technical changes]
- [Important implementation details]

## Testing
- [x] All tests pass
- [x] No clippy warnings
- [x] Code formatted with cargo fmt

## Test Plan
- [How to verify the changes work]
- [Any manual testing steps]
```

## Important Notes

- NEVER skip quality checks
- NEVER commit with failing tests or clippy warnings
- ALWAYS re-run ALL checks after fixing issues
- Be thorough and methodical
- The quality check loop is mandatory, not optional

## Success Criteria

You have successfully completed this skill when:
- [ ] Code is formatted with `cargo fmt`
- [ ] Clippy passes with no warnings
- [ ] All tests pass
- [ ] Changes are committed with a clear message
- [ ] Git status shows clean working tree (or unpushed commit)
