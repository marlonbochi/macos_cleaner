# Feature Specification: Batch Analysis of Selected Items

**Feature Branch**: `003-batch-ai-analysis` (created from `002-ai-risk-analysis`)

**Created**: 2026-10-14

**Status**: Implemented and merged into `main`

**Input**: User description (translated from Portuguese): "Add a selector/checkbox to the screen
that lists the files and, from that selection, send the selected files' details to the AI in a
single request instead of one per file, making the search smaller and using fewer tokens."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Select and analyse several items at once (Priority: P1)

In the largest-items list the user ticks checkboxes (or the one for the whole page) and clicks
"Analyse selected with AI". The items are analysed together and each row gets its risk badge; each
item's panel shows the full analysis without a new request.

**Acceptance Scenarios**:

1. **Given** 2 selected items, **When** the user analyses them, **Then** 1 AI request is made and
   both items get an analysis.
2. **Given** 45 selected items, **When** analysed, **Then** 3 requests are made (20 + 20 + 5).
3. **Given** items already analysed and unchanged in the selection, **When** analysed, **Then** they
   are not sent again.
4. **Given** the AI omits or answers invalidly for one item, **When** the batch finishes, **Then**
   only that item uses the local rules and the user is told.
5. **Given** protected items, **When** listed, **Then** their checkbox is disabled.

## Requirements *(mandatory)*

- **FR-301**: The list MUST allow selecting items individually and the whole page; the selection
  persists across pages and filters of the same scan and is cleared when the scan changes.
- **FR-302**: The system MUST group pending items into requests of up to 20 items, sending the
  system prompt and schema once per request, with compact JSON.
- **FR-303**: Folders in a batch MUST send at most 5 children (15 in single analyses).
- **FR-304**: The answer MUST be validated per item, matched by `id`; failures are isolated.
- **FR-305**: At most 200 items per analysis action.

## Success Criteria *(mandatory)*

- **SC-301**: Analysing N selected items makes ⌈N/20⌉ requests instead of N (tested).
- **SC-302**: For 20 items, the system prompt and schema are sent once instead of 20 times.
