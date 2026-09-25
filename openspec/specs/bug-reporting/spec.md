# bug-reporting Specification

## Purpose
Defines the in-app path from the Help menu to a filed GitHub issue: the bug
report window, the subject and description it collects, the app diagnostics it
attaches, and how a report is delivered - submitted with the user's own
credentials when the forge tool is available, and degraded to a pre-filled
browser page otherwise.

## Requirements

### Requirement: The bug report window is opened from the Help menu

The Help menu SHALL carry a "Report a Bug…" item that opens the bug report
window as a dialog attached to the active window.

Choosing the item while the window is already open SHALL bring the existing
window forward rather than opening a second one. After it has been closed,
choosing the item SHALL open it again.

#### Scenario: Opening the window

- **WHEN** the user chooses Help > Report a Bug…
- **THEN** the bug report window opens attached to the active window

#### Scenario: One window at a time

- **WHEN** the bug report window is already open and the user chooses Help >
  Report a Bug… again
- **THEN** the existing window comes forward and no second window opens

### Requirement: The window collects subject, description and diagnostics

The bug report window SHALL carry:

- A subject field that accepts a single line of text.
- A description text area that accepts multiple lines.
- An app diagnostics section, filled in automatically and not editable, that
  shows the version and build identifier of the running binary (the same
  values the About window shows, derived from the binary rather than from
  maintained text), the host operating system and version, and the hardware
  architecture.
- The diagnostics SHALL additionally state whether the tool Knot uses to reach
  the forge is available and authenticated. The three distinct states - not
  installed, not authenticated, ready - SHALL be told apart, because they have
  different consequences for how a report can be submitted.
- Cancel and Report controls.

The window SHALL present the diagnostics values as selectable text, so a user
can copy them into a report filed out of band without transcribing them.

#### Scenario: The window shows what will accompany the report

- **WHEN** the user opens the window on a macOS build of a released binary
- **THEN** the version, build identifier, OS name and version, architecture
  and forge-readiness are shown, filled in from the running binary

#### Scenario: Diagnostics are copyable

- **WHEN** the user opens the window and selects the diagnostics text
- **THEN** the text can be copied, so it can be pasted into a report filed by
  other means

### Requirement: Report requires a subject and a description

Report SHALL be disabled while the subject is empty or consists only of
whitespace, or while the description is empty or consists only of whitespace,
and SHALL become enabled as soon as both are present.

Escape SHALL cancel the window, discarding what has been typed.

#### Scenario: An empty subject blocks submission

- **WHEN** the window is open with a description but no subject
- **THEN** Report is disabled

#### Scenario: An empty description blocks submission

- **WHEN** the window is open with a subject but no description
- **THEN** Report is disabled

#### Scenario: Escape cancels

- **WHEN** the user has typed a subject and description and presses Escape
- **THEN** the window closes and no report is filed

### Requirement: A report is submitted with the user's credentials

When the forge tool is available and authenticated, choosing Report SHALL file
a GitHub issue on the app's repository: the subject as the issue title, and a
body that carries the description followed by the diagnostics section.

Knot SHALL use the credentials the user has already given the forge tool; it
SHALL NOT ask the user for a token, store one, or prompt for anything the tools
it shells out to does not need.

While the issue is being filed, the window SHALL indicate that the submission
is in progress and SHALL prevent a second Report from being submitted at the
same time.

On success, the window SHALL confirm that the issue was filed and SHALL close.

#### Scenario: The issue is filed

- **WHEN** the forge tool is ready and the user fills both fields and chooses
  Report
- **THEN** an issue titled with the subject is created on the app's repository,
  its body carrying the description and the diagnostics
- **AND** the window says the issue was filed and closes

#### Scenario: No credentials are asked for

- **WHEN** the user files a report
- **THEN** Knot never asks for a token or stores one, using the credentials the
  forge tool already holds

#### Scenario: Double submission is prevented

- **WHEN** a submission is in progress
- **THEN** Report is disabled until that submission finishes

### Requirement: A report degrades to a browser page when the forge is untrusted

When the forge tool is not installed, or is installed but not authenticated,
choosing Report SHALL open the GitHub issue-compose page for the app's
repository in the user's default browser, pre-filled with the subject as the
title and a body carrying the description and the diagnostics. Nothing the user
typed, and no diagnostic, SHALL be lost.

The window SHALL make clear, in the process, that the report was not filed
automatically and is ready to be posted in the browser.

If the browser page cannot be opened, the failure SHALL be reported and the
window SHALL keep the user's subject and description so nothing is lost.

#### Scenario: The forge tool is not installed

- **WHEN** the forge tool is absent and the user chooses Report
- **THEN** the issue-compose page opens in the browser with the subject,
  description and diagnostics pre-filled, and the window says the report is
  ready to post

#### Scenario: The users have not authenticated

- **WHEN** the forge tool is installed but not authenticated and the user
  chooses Report
- **THEN** the same browser fallback happens, with the same pre-filled content

#### Scenario: The browser cannot be opened

- **WHEN** the forge tool is unavailable and the browser page cannot be opened
- **THEN** the failure is reported and the subject and description remain in
  the window, un-submitted and uncopied

### Requirement: A failed submission keeps the report

When a submission is attempted through the forge tool and the tool fails - the
request errors rather than returning a filed issue - the window SHALL report
the failure and SHALL leave the subject, description and diagnostics in place,
so the user can retry or copy them. A failure SHALL NOT wipe the report.

The user SHALL be able to retry without retyping: after a failure, Report SHALL
be enabled again and choosing it SHALL attempt the submission again.

#### Scenario: The forge tool errors

- **WHEN** the forge tool fails while filing an issue
- **THEN** the window reports the failure, keeps the subject and description,
  and offers a working Report to retry

### Requirement: The window's text is localized

Every piece of user-facing text the window shows - the menu item label, the
field labels, the diagnostics labels, the state the forge availability is
described in, the success and failure messages, and the Cancel and Report
controls - SHALL come from the localization catalog rather than being written
inline, so the window can be translated without changing it.

Values that are not prose - the version number, the build identifier, the OS
name and version, and the architecture - are not translated, but the text
around them SHALL be.

#### Scenario: Every label resolves through the catalog

- **WHEN** a validation runs that renders every localizable string the window
  can show
- **THEN** each one resolves to a catalog entry, so no label is missing or
  inline
