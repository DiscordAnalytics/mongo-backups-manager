# Contributing to MongoDB Backups Manager
Thank you for your interest in contributing! We welcome all kinds of help, from bug reports to feature suggestions and code contributions.

---

## Report a Bug
Found a bug? Help us squash it! 

1. **Check for existing issues:** Before opening a new one, please search our [Issue Tracker](link-to-issues) to see if it has already been reported.
2. **Open an Issue:** If the bug is new, open an issue using the provided template. Please include as much detail as possible (environment, steps to reproduce, and logs).
3. **Submit a Pull Request:** If you have a fix ready and feel comfortable, feel free to open a Pull Request (PR) linked to that issue.

## Feature Requests and Questions
If you have questions or ideas that aren't bugs, please use our **Discussions** tab:

* **Questions:** If you are encountering an issue that isn't a bug, a community member or a member of the team will try to assist you there.
* **Feature Ideas:** Have an idea to make this tool better? Open a Discussion to gather feedback from the community. After internal review, we may convert the discussion into an official issue and plan it for a future update.

## Developer Notice
This section is for developers who wish to contribute code. In addition to these rules, please adhere to our [Code of Conduct](/.github/CODE_OF_CONDUCT.md).

### Before Opening a Pull Request
To ensure a smooth review process, please follow these steps:

1.  **Open an Issue first:** Describe the bug or feature before writing code.
2.  **Link your PR:** Use [linking keywords](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/using-keywords-in-issues-and-pull-requests) (e.g., `Closes #123`) in your PR description.
3.  **Draft PRs:** If you start working on an issue, open a **Draft Pull Request** early. This signals to other contributors that the task is being handled.
4.  **Stay Updated:** We recommend frequently rebasing or merging the main branch into your feature branch to avoid complex merge conflicts.

> [!IMPORTANT]
> To maintain project quality, Pull Requests that do not follow the commit formatting rules or lack a linked issue may be closed or requested for revision.

### Commit Message Format
We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification. This allows us to automate changelogs and versioning.

Each commit must follow this structure:
```
<type>(<scope>): <message>
```

* **Types:** `feat`, `fix`, `docs`, `refactor`, `test`, `chore`.
* **Scope:** The specific part of the codebase (e.g., `config`, `crypto`, `logger`).
* **Message:** A brief, imperative description (e.g., "add backup encryption").

#### Breaking Changes
If a commit introduces a breaking change, add a `!` after the type/scope and include a `BREAKING CHANGE` footer:
```text
feat(api)!: Add the `force` argument to write_config() function

BREAKING CHANGE: The `force` parameter is now required for all configuration writes.
```

---

Thank you for helping us make **MongoDB Backups Manager** better! ❤️