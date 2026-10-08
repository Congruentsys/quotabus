---
id: SIG-003
title: "Q4: undocumented usage endpoints — off by default, on for the fleet, always labelled?"
type: signal
status: backlog
priority: medium
assignee: null
created: 2026-10-08
tags: [captain-decision, design]
depends_on: []
---

# Q4: undocumented usage endpoints — off by default, on for the fleet, always labelled?

Design §10 Q4. Undocumented sources (Anthropic oauth/usage, Copilot copilot_internal/user, z.ai biz endpoints). **Recommendation:** off by default in the shipped config; on in the fleet's config; every row from one labelled `source = undocumented`, so it degrades to unknown, never to a false all-clear.