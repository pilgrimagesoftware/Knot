# Spec Delta

## ADDED Requirements

### Requirement: Drawing a terminal frame does not enumerate installed fonts

Drawing or sizing the terminal surface SHALL NOT enumerate the fonts
installed on the machine. The family the surface draws in SHALL be resolved
when the configured terminal font name is first seen and reused for every
subsequent frame that asks for the same name, so per-frame cost is
independent of how many fonts are installed.

Resolution behaviour is unchanged: a configured name the text system cannot
resolve SHALL still fall back to the embedded default monospace family
rather than to the proportional UI font.

The Swift app does not have this cost - it hands the family name to a native
terminal surface and never asks the text system which families exist.

#### Scenario: Repeated frames resolve the same family once

- **WHEN** the terminal surface draws many frames with the terminal font
  setting unchanged
- **THEN** the installed font families are enumerated at most once across
  those frames

#### Scenario: Changing the terminal font takes effect

- **WHEN** the user changes the terminal font name in settings
- **THEN** the next frame draws in the newly configured family, resolving it
  against the installed families

#### Scenario: An uninstalled family still falls back

- **WHEN** the configured terminal font name is not installed
- **THEN** the surface draws in the embedded default monospace family
- **AND** subsequent frames reuse that fallback without enumerating again
