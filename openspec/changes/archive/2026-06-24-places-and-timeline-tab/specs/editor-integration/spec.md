# Delta for editor-integration

## ADDED Requirements

### Requirement: Context Menu "Add to Place"

The context menu MUST show an "Add to place" option when the user selects text in the editor. Selecting this option SHALL prompt the user to choose an existing place or create a new one. The selected text SHALL be appended to the chosen place's `description` field via `actualizar_lugar`.

#### Scenario: Append selected text to existing place

- GIVEN the user has selected "la torre del vigía" in the editor
- WHEN the user right-clicks and chooses "Add to place" → selects "Torre Norte"
- THEN `actualizar_lugar` is invoked with the place's current description + "\n" + selected text
- AND a success toast confirms the update

#### Scenario: Create new place from context menu

- GIVEN the user has selected text and right-clicks
- WHEN the user chooses "Add to place" → "Create new"
- THEN a prompt collects name and optional initial description
- AND `crear_lugar` is invoked with the selected text as the description
- AND the new place appears in the Places tab

#### Scenario: Option hidden when no text is selected

- GIVEN the cursor is placed in the editor with no active selection
- WHEN the context menu opens
- THEN "Add to place" does NOT appear in the menu

#### Scenario: Cancel dismisses the prompt

- GIVEN the "Add to place" prompt is open
- WHEN the user cancels or clicks outside
- THEN no IPC command is invoked
- AND the editor state is unchanged
