# Role-Play Demo Workflow

Multi-agent conversation system with semantic analysis and director control.

## Components

| Component | Role |
|-----------|------|
| **Orchestrator** | Rust SDK managing turn flow, session continuity, and state |
| **Agent A / B** | Two Sonnet agents with persistent personas |
| **Haiku Analyzer** | Out-of-band semantic evaluator with structured output |
| **Director** | Human controlling flow via commands and notes |
| **Scene State** | File-based meters, beats, and ephemeral notes |
| **Hooks** | Context injection on threshold crossings |

## Session Management

Each participant maintains independent session continuity:

```
Agent A: session_id_a → resumed each time A speaks
Agent B: session_id_b → resumed each time B speaks
Analyzer: analyzer_session_id → resumed for consistent analysis style
```

**Turn flow:**
1. Agent A speaks → capture session_id_a
2. Analyzer evaluates A's dialogue → capture analyzer_session_id
3. Agent B speaks (receives "A said: ...") → capture session_id_b
4. Analyzer evaluates B's dialogue → resume analyzer_session_id
5. Loop continues

Agents see their full conversation history via session resume. They don't see the other agent's internal reasoning—only what was said.

## Main Loop

**Two phases per iteration:**

**Phase 1: Turn Execution**
- Determine active agent (A or B alternating)
- Build prompt with other agent's last response
- Create agent client with `resume(session_id)`
- Stream response, render markdown
- Capture new session_id from Result message
- Spawn Haiku analyzer with dialogue + current state
- Write updated state from analyzer output

**Phase 2: Director Input**
- Wait for slash command or free text
- Commands either loop back (`/help`, `/status`) or advance scene (`/start`)
- Free text becomes director interjection for next turn
- Ctrl+C pauses for director input

## Haiku Analyzer

Runs after each agent turn, outside the main conversation:

**Input:**
- Agent's dialogue (text only)
- Current scene state (tension, heat, beat)
- Director note if present (for alignment check)

**Output (structured JSON):**
- `tension.to` / `tension.reason`
- `heat.to` / `heat.reason`
- `beat.current` / `beat.changed`
- `director_aligned` (did agent follow the note?)

The analyzer maintains its own session for consistent narrative tracking across turns.

## Scene State

File-based state in `.claude/scene-state/`:

| File | Purpose |
|------|---------|
| `meters/tension.txt` | Narrative stakes (1-10) |
| `meters/heat.txt` | Romantic intensity (1-5) |
| `beat.txt` | Current narrative beat |
| `analysis.json` | Full analyzer output with reasons |
| `notes/{agent}.txt` | Per-agent director notes (ephemeral) |

State is written by the orchestrator after each analyzer run.

## Director Commands

| Command | Effect |
|---------|--------|
| `/start [N]` | Begin or add N turns |
| `/stop` | Pause scene |
| `/turns N` | Set remaining turns |
| `/say <agent> "msg"` | Write ephemeral note for agent |
| `/tension N` | Override tension (1-10) |
| `/heat N` | Override heat (1-5) |
| `/beat X` | Override beat |
| `/hitl` | Enable human-in-the-loop mode |
| `/auto` | Disable HITL, run continuously |
| `/status` | Display current state |

## Hook System

**UserPromptSubmit hook** (`scene-state-hook.js`) injects context when:

1. **Meter threshold crossing** — Tension or heat moves between normal/alert/critical
2. **Beat transition** — Narrative beat changes
3. **Director note present** — Ephemeral instruction for this agent

The hook compares current state against `.previous-state.json` to detect changes. No output if nothing crossed a threshold.

**Injection format:**
```
━━━ SCENE STATE UPDATE ━━━
⚠️ Alert: [threshold crossed with guidance]
🎭 Beat: [new beat with context]

━━━ DIRECTOR'S NOTE ━━━
[One-time instruction, deleted after injection]
```

## Director Notes Flow

1. Director types `/say luna "Show vulnerability"`
2. Orchestrator writes to `notes/luna.txt`
3. Next time Luna's agent runs, hook reads and injects note
4. Note file deleted immediately (ephemeral)
5. Haiku evaluates if Luna's response aligned with direction

Notes are firewall-delimited—each agent only sees notes intended for them.

## Dynamic Guidance

The hook prefers semantic reasons from `analysis.json` over hardcoded level descriptions:

- **Dynamic**: "Luna's charm successfully de-escalates confrontation" (from Haiku)
- **Fallback**: "Tension is high (7/10)" (hardcoded level)

Injection logs track `guidanceSource: "dynamic"` vs `"fallback"` for debugging.

## State Machines

**Turn advancement:**
```
running && turns > 0 → execute turn → decrement turns
turns == 0 → pause (running = false)
HITL mode → pause after each turn
Auto mode → continue immediately
```

**Director input:**
```
/start or Enter → advance scene
/help, /status, /say → loop back for more input
Ctrl+C → pause, enter director input
```

## Key Design Principles

- **Stateless agents, stateful sessions** — Clients are per-turn, sessions persist
- **Change-only injection** — Hooks output only on meaningful state changes
- **Ephemeral directives** — Director notes consumed once, never repeated
- **Out-of-band analysis** — Haiku runs parallel to main conversation
- **Structured output** — Analyzer returns guaranteed-valid JSON
- **File-based state** — All state readable/writable via filesystem
