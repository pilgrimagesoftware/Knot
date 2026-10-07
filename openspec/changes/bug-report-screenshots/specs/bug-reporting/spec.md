# Spec Delta

## ADDED Requirements

### Requirement: A report can carry screenshots

The window SHALL let the user choose screenshots to accompany a report, for a
bug or a feature request alike, through the platform's file picker, with
more than one selectable at a time. None SHALL be chosen by default.

Only image files the forge renders inline (PNG, JPEG, GIF) SHALL be kept, up
to a fixed number. A chosen file that is not one, or that would exceed the
number, SHALL be skipped, and the window SHALL say how many were skipped.
Each kept screenshot SHALL be listed by file name, with a control that
removes it.

The forge offers no way to upload a file with an issue, so a screenshot
SHALL NOT be sent anywhere by Knot itself. The issue body SHALL name each
chosen screenshot under its own heading, as something the reporter will add.
Knot SHALL then put the files where the user can drag them onto the issue:

- **Filed through the forge tool:** Knot SHALL open the filed issue's page
  in the browser and show the screenshots in the platform's file manager,
  and the confirmation SHALL say to drag them onto the issue.
- **Browser fallback:** Knot SHALL show the screenshots in the file manager
  beside the compose page, and the status SHALL say to drag them onto the
  page.

Where the platform has no file manager to show them in, the names in the
body SHALL still be written, and the message SHALL say to attach them by
hand.

#### Scenario: Choosing screenshots

- **WHEN** the user chooses two PNG files and a text file through Add
  Screenshots
- **THEN** the two PNGs are listed by name, and the window says one file was
  skipped

#### Scenario: Filing with screenshots

- **WHEN** the forge tool is ready and the user files a report with a
  screenshot chosen
- **THEN** the issue body names the screenshot, the filed issue's page opens
  in the browser, the screenshot is shown in the file manager, and the
  confirmation says to drag it onto the issue

#### Scenario: The browser fallback with screenshots

- **WHEN** the forge tool is not ready and the user files a report with a
  screenshot chosen
- **THEN** the compose page's body names the screenshot, the screenshot is
  shown in the file manager, and the status says to drag it onto the page

#### Scenario: No screenshot unasked

- **WHEN** the user files a report without choosing a screenshot
- **THEN** the body has no screenshots heading, and nothing is shown in the
  file manager

#### Scenario: Removing a screenshot

- **WHEN** the user removes a listed screenshot before filing
- **THEN** the report neither names it nor shows it
