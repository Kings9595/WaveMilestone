# WaveMilestone Label Management

This document provides guidelines for managing GitHub labels in the WaveMilestone repository. Proper label management helps with issue tracking, project prioritization, and contributor engagement.

## Why Labels Matter

Labels serve as visual cues that help:
- **Prioritize work** – Quickly identify high-priority issues
- **Guide contributors** – New contributors can find "good first issue" labels
- **Track progress** – Monitor which issues are in each stage
- **Automate workflows** – Trigger actions based on label changes

## Label Categories

### Issue Status
- `bug` – Issues that represent bugs in the codebase
- `enhancement` – New feature requests
- `documentation` – Documentation improvements or requests
- `question` – Questions that need clarification
- `help wanted` – Issues where contributors are welcome to help
- `good first issue` – Issues suitable for new contributors

### Workflow Stages
- `triage` – Issues awaiting initial review
- `in progress` – Issues currently being worked on
- `blocked` – Issues that are blocked on dependencies or external factors
- `needs review` – Issues ready for code review
- `under review` – Issues currently under review
- `done` – Completed issues

### Technical Categories
- `contract` – Issues related to smart contract logic
- `testing` – Issues related to testing
- `security` – Security-related issues
- `performance` – Performance optimization issues
- `documentation` – Documentation-related issues
- `infrastructure` – Infrastructure or tooling issues

### Milestone Related
- `milestone-1` – Issues for milestone 1
- `milestone-2` – Issues for milestone 2
- `milestone-3` – Issues for milestone 3

## Label Management Best Practices

### 1. Be Consistent

Use consistent naming conventions:
- Use lowercase with hyphens for multi-word labels
- Avoid special characters except hyphens
- Keep labels short and descriptive
- Use singular nouns (e.g., `bug`, not `bugs`)

### 2. Use Color Wisely

Choose colors that are:
- **Distinct** – Easy to differentiate from other labels
- **Accessible** – Good contrast for colorblind users
- **Professional** – Not too bright or distracting

### 3. Limit Label Usage

Keep the number of active labels manageable:
- **Active labels**: 10-15 labels that are currently in use
- **Archive labels**: Move old or unused labels to an archive
- **Review regularly**: Clean up unused labels quarterly

### 4. Label Issues Appropriately

When opening an issue:
- **Select the most relevant primary label** first
- **Add secondary labels** if applicable
- **Remove irrelevant labels** before closing

When updating an issue:
- **Update labels** as the issue progresses
- **Add `in progress`** when you start working on it
- **Remove `triage`** once reviewed

### 5. Use Labels for Automation

Leverage labels to trigger workflows:
- **Auto-assign** based on labels (e.g., `contract` → contract reviewer)
- **Auto-close** issues with `duplicate` or `won't fix` labels
- **Auto-milestone** issues based on milestone labels
- **Auto-notify** teams based on category labels

## Label Workflow

### Opening an Issue

1. **Choose the right issue type**
   - `bug` for actual bugs
   - `enhancement` for new features
   - `question` for questions

2. **Add relevant technical labels**
   - `contract`, `testing`, `security`, etc.

3. **Add status label**
   - Default to `triage` for new issues

4. **Assign milestone** (if applicable)
   - Use `milestone-1`, `milestone-2`, etc.

### Processing Issues

1. **Review and triage**
   - Remove `triage` label
   - Add `needs review` or `in progress`

2. **During development**
   - Change to `in progress`
   - Remove `needs review`

3. **After completion**
   - Add `done` label
   - Remove all other labels

### Closing Issues

1. **Fixed bugs**
   - Add `done` label
   - Reference the fix commit

2. **Closed as duplicate**
   - Add `duplicate` label
   - Reference the original issue

3. **Won't fix**
   - Add `won't fix` label
   - Provide reasoning

## Label Usage Examples

### Example 1: Bug Report
```
Title: Contract panics when releasing bounty with zero amount
Labels: bug, contract, needs review
```

### Example 2: Feature Request
```
Title: Add support for multiple assets in milestone pool
Labels: enhancement, contract, milestone-2
```

### Example 3: Documentation Issue
```
Title: Missing example in README for create_milestone_pool
Labels: documentation, good first issue
```

## Tools for Label Management

### GitHub CLI

```bash
# List all labels
gh label list

# Create a new label
gh label create "good first issue" --color ffffff --description "Great for beginners"

# Update a label
gh label edit "in progress" --color 0075ca

# Delete a label
gh label delete "old-label"
```

### GitHub Actions

Use labels to trigger workflows:

```yaml
# .github/workflows/issue-triage.yml
name: Issue Triage
on:
  issues:
    types: [opened]

jobs:
  triage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/github-script@v6
        with:
          script: |
            // Auto-assign based on labels
            if (issue.labels.includes('contract')) {
              await github.rest.issues.addLabels({
                issue_number: issue.number,
                owner: context.repo.owner,
                repo: context.repo.repo,
                labels: ['contract-review']
              });
            }
```

## Label Archive

Move old or unused labels to an archive:

```bash
# Rename archived labels (prefix with "archive/")
gh label edit "old-label" --name "archive/old-label"

# Create a label for archived items
gh label create "archive" --color 808080 --description "Archived labels"
```

## Monitoring Label Usage

Track label usage with GitHub Insights:

1. **Check label distribution** in the Issues tab
2. **Monitor label changes** with GitHub Actions
3. **Review quarterly** and clean up unused labels

## Common Pitfalls to Avoid

1. **Over-labeling** – Too many labels make filtering difficult
2. **Under-labeling** – Missing important context
3. **Inconsistent colors** – Hard to scan visually
4. **Unused labels** – Clutter the interface
5. **Confusing labels** – Make it hard to understand issue status

## References

- [GitHub Documentation: Managing Labels](https://docs.github.com/en/issues/using-labels-and-milestones-to-track-work/managing-labels)
- [GitHub CLI: Labels](https://cli.github.com/manual/gh_label)
- [GitHub Actions: Issues Trigger](https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows#issues)
