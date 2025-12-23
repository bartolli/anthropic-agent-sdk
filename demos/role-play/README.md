# Claude Role-Play

Two AI agents with distinct personas engage in dynamic, director-controlled conversations.

## What It Does

Drop two Claude agents into a scene - a detective and an artist, rivals at a poker table,
old friends reuniting - and watch them improvise. A third agent (Haiku) silently analyzes
the emotional dynamics, tracking tension, chemistry, and narrative beats.

You're the director. Whisper notes to individual characters. Crank up the tension.
Steer the story. Or sit back and let it unfold.

## Features

- **Persistent personas** - Each agent maintains character across turns via session resume
- **Semantic analysis** - Haiku evaluates every line for tension, heat, and beat changes
- **Director control** - Inject notes, override meters, pause/resume the scene
- **Human-in-the-loop** - Step through turn-by-turn or let it run
- **Structured output** - Analyzer returns guaranteed-valid JSON for state updates
- **Threshold hooks** - Context injected only when meaningful state changes occur

## Installation

```bash
cargo install claude-role-play
```

Requires [Claude Code](https://www.npmjs.com/package/@anthropic-ai/claude-code) CLI installed.

**Cost note:** This demo runs multiple Claude agents per turn (two conversants + analyzer).
Recommended for Claude Pro/Max subscribers. API key usage can accumulate quickly.

## Quick Start

```bash
claude-role-play \
  --persona-a detective.txt \
  --persona-b artist.txt \
  --name-a "Detective Rourke" \
  --name-b "Luna" \
  --scene "Investigating a mysterious art heist" \
  --turns 4 \
  --model sonnet
```

**Model options:** `opus` (most capable), `sonnet` (default), `haiku` (fastest/cheapest)

**Analyzer model:** The semantic analyzer defaults to `haiku` for cost efficiency. Power users can
override with `--analyzer-model sonnet` for deeper analysis at higher cost.

Create persona files with character descriptions, or use the included examples from the [demos/role-play/personas](https://github.com/bartolli/anthropic-agent-sdk/tree/main/demos/role-play/personas) directory.

## Director Commands

| Command | Effect |
|---------|--------|
| `/start [N]` | Begin or add N turns |
| `/say luna "msg"` | Whisper a note to Luna (one-time) |
| `/tension 8` | Override tension meter (1-10) |
| `/heat 3` | Override heat/chemistry meter (1-5) |
| `/hitl` | Human-in-the-loop mode (pause after each turn) |
| `/status` | Show current scene state |

## Why This Exists

Built with [anthropic-agent-sdk](https://crates.io/crates/anthropic-agent-sdk) to demonstrate
that Claude Code can power applications beyond coding. This is a reference implementation for:

- Multi-turn agentic workflows with session continuity
- Parallel out-of-band analysis (main conversation + evaluator)
- Human-in-the-loop control patterns
- File-based state management
- Hook-driven context injection

See [WORKFLOW.md](WORKFLOW.md) for the complete technical architecture.

## License

MIT
