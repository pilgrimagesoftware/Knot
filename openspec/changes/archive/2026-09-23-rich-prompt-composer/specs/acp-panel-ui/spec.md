# Spec Delta

## MODIFIED Requirements

### Requirement: Input area context attachment
The input area SHALL provide an add-context control that lets the user
attach files or images to the next message.

Attached context SHALL have two presences, not one: an entry in the strip
above the input carrying its name and, for an image, its thumbnail; and a
styled reference at the caret in the buffer, giving the attachment a position
in the prompt. The two SHALL stay in step — removing either one removes the
attachment.

This holds however the context arrived: the add-context control, a drag from
the Finder, or a pasted image. This diverges from the Swift reference, whose
attachments appear only in the strip and have no position in the text.

#### Scenario: Attach a file
- **WHEN** the user activates the add-context control and selects a file
- **THEN** the file is attached to the pending message and shown in the
  input area before send

#### Scenario: An attachment has a place in the text
- **WHEN** the user attaches a file with the caret mid-sentence
- **THEN** a styled reference to it appears at the caret and its entry appears
  in the strip above the input

#### Scenario: Removing the strip entry removes the reference
- **WHEN** the user removes an attachment's entry from the strip
- **THEN** its reference is gone from the buffer and the attachment is not
  sent

#### Scenario: Deleting the reference removes the strip entry
- **WHEN** the user deletes an attachment's reference from the buffer
- **THEN** its entry is gone from the strip and the attachment is not sent
