# Feature Specification: AI Diagnosis of the Largest Items

**Feature Branch**: `008-ai-diagnosis`

**Created**: 2026-10-14

**Status**: Implemented

**Input**: User description (translated from Portuguese): "When a scan finishes, send the 40
largest items to the AI and add a tab listing them in the same layout as the largest items, called
AI diagnosis, saying what the AI thinks of each one. Only show this screen and the AI features when
an AI key has been saved."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Automatic diagnosis after a scan (Priority: P1)

With an AI provider configured, the user runs a scan. As soon as it completes, the largest items
are sent to the AI in the background. The "AI diagnosis" tab lists them, largest first, each with
its risk badge and a one-line summary of what the AI thinks, plus the space per risk level.

**Acceptance Scenarios**:

1. **Given** an AI provider is configured, **When** a scan completes, **Then** the diagnosis runs
   once, with ⌈N/20⌉ AI requests for N items.
2. **Given** a cancelled scan, or simply opening the app, **When** nothing else happens, **Then**
   no AI request is made.
3. **Given** the diagnosis finished, **When** the user opens the tab, **Then** every item shows its
   risk level and summary, and the totals per level add up the items' sizes.
4. **Given** items analysed before and unchanged, **When** the diagnosis runs again, **Then** they
   are not sent again unless the user asks to analyse everything again.
5. **Given** a folder in the list, **When** the user clicks "Open", **Then** the largest-items list
   opens inside that folder.

### User Story 2 - AI features only when AI is configured (Priority: P1)

**Acceptance Scenarios**:

1. **Given** no provider is configured (or a cloud provider without a saved key), **When** the user
   looks at the app, **Then** the "AI diagnosis" tab, the selection checkboxes and "Analyse
   selected with AI" are hidden; the per-item analysis keeps working with the local rules.
2. **Given** the user saves a provider with its key (or Ollama, which needs none), **When** the
   settings are saved, **Then** the tab and the batch features appear without restarting.

## Requirements *(mandatory)*

- **FR-801**: The diagnosis MUST cover the 40 largest items of the scan that are not protected and
  are not "wrapper" folders (a folder whose largest catalogued child holds ≥ 90% of its size), so a
  chain such as `Containers → docker → … → Docker.raw` is sent once, as its innermost item.
- **FR-802**: The diagnosis MUST run automatically only on a running → completed transition of a
  scan, and only when an AI provider is configured.
- **FR-803**: The diagnosis MUST reuse batch analysis (FR-302..FR-304): up to 20 items per request,
  cached per item state, per-item fallback to the local rules with the reason shown.
- **FR-804**: Running the diagnosis without a configured provider MUST fail with "not configured"
  and send nothing.
- **FR-805**: The tab MUST list the items in the largest-items layout (no selection column) with the
  AI summary per row, totals of space per risk level, and the full analysis panel on demand.
- **FR-806**: The "AI diagnosis" tab, the selection checkboxes and batch analysis MUST only be shown
  when an AI provider is configured.

## Success Criteria *(mandatory)*

- **SC-801**: For a typical home folder, the diagnosis is two AI requests (40 items) and finishes in
  about a minute without blocking the UI.
- **SC-802**: Re-running the diagnosis on an unchanged catalogue makes no AI request.
