# Tasks: AI diagnosis of the largest items

- [X] D001 `DiagnosisService`: candidate picking (protected and wrapper folders skipped), `latest`, `run` requiring a configured AI (tests)
- [X] D002 IPC commands `get_ai_diagnosis` and `run_ai_diagnosis`; `DiagnosisItemDto` exported to TypeScript
- [X] D003 Diagnosis store: load, run (one run per scan at a time), pending items, fallbacks, totals per risk level (tests)
- [X] D004 Auto-run on a running → completed scan transition when AI is configured (tests)
- [X] D005 "AI diagnosis" view: totals, table with AI summaries, run / analyse again, folders open in the list (tests)
- [X] D006 Hide the tab, selection checkboxes and batch analysis without a configured AI (tests)
- [X] D007 English and Brazilian Portuguese strings
- [ ] D008 Manual validation with a real key on a full-disk scan
