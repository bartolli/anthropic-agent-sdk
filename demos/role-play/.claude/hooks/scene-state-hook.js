#!/usr/bin/env node

/**
 * Scene State Hook (UserPromptSubmit)
 *
 * Reads scene state and injects context ONLY when:
 * - A meter crosses a threshold
 * - A beat changes
 * - There's a director's note (ephemeral)
 *
 * Uses CLAUDE_PROJECT_DIR for path resolution.
 * Uses shared/matcher.js for trigger matching.
 */

import fs from 'fs';
import path from 'path';
import { matchTriggers, getThresholdStatus } from './shared/matcher.js';

function main() {
  let input = '';

  process.stdin.on('data', chunk => {
    input += chunk;
  });

  process.stdin.on('end', () => {
    try {
      const data = JSON.parse(input);
      const projectDir = process.env.CLAUDE_PROJECT_DIR;

      // Debug: log to stderr (visible in claude --debug or ctrl+o verbose mode)
      console.error(`[scene-state] CLAUDE_PROJECT_DIR=${projectDir}`);
      console.error(`[scene-state] CLAUDE_AGENT_NAME=${process.env.CLAUDE_AGENT_NAME || 'not set'}`);
      console.error(`[scene-state] prompt preview: ${(data.prompt || '').slice(0, 50)}...`);

      // Breadcrumb: write timestamp to verify hook is running
      const breadcrumbPath = projectDir
        ? path.join(projectDir, '.claude', 'logs', 'hook-breadcrumb.txt')
        : '/tmp/scene-state-hook-breadcrumb.txt';
      try {
        fs.mkdirSync(path.dirname(breadcrumbPath), { recursive: true });
        fs.appendFileSync(breadcrumbPath, `[scene-state] ${new Date().toISOString()} - prompt: ${(data.prompt || '').slice(0, 30)}...\n`);
      } catch (e) {
        console.error(`[scene-state] breadcrumb error: ${e.message}`);
      }

      if (!projectDir) {
        console.error('[scene-state] No CLAUDE_PROJECT_DIR, exiting');
        process.exit(0);
      }

      const claudeDir = path.join(projectDir, '.claude');
      const stateDir = path.join(claudeDir, 'scene-state');
      const rulesPath = path.join(claudeDir, 'scene-rules.json');

      // Load scene rules
      if (!fs.existsSync(rulesPath)) {
        process.exit(0);
      }

      const rules = JSON.parse(fs.readFileSync(rulesPath, 'utf-8'));

      // Load current state
      const state = loadState(stateDir);
      const previousState = loadPreviousState(stateDir);

      // Load Haiku's analysis for dynamic guidance
      const analysisPath = path.join(stateDir, 'analysis.json');
      let analysis = null;
      if (fs.existsSync(analysisPath)) {
        try {
          analysis = JSON.parse(fs.readFileSync(analysisPath, 'utf-8'));
        } catch {
          // Ignore parse errors, fall back to hardcoded levels
        }
      }

      // Build output only for notable changes
      const alerts = [];

      // Check meter thresholds
      for (const [meterName, meterConfig] of Object.entries(rules.meters || {})) {
        const value = state.meters[meterName] || meterConfig.default;
        const prevValue = previousState.meters[meterName] || meterConfig.default;
        const status = getThresholdStatus(value, meterConfig.thresholds);
        const prevStatus = getThresholdStatus(prevValue, meterConfig.thresholds);

        // Only alert on threshold crossing
        if (status !== prevStatus && status !== 'normal') {
          const levelKey = getLevelKey(value, meterConfig.range);
          const levelDesc = meterConfig.levels?.[levelKey] || '';

          // Prefer dynamic reason from Haiku's analysis, fall back to hardcoded levels
          const dynamicReason = analysis?.[meterName]?.reason;
          const guidance = dynamicReason || levelDesc;
          const guidanceSource = dynamicReason ? 'dynamic' : 'fallback';

          if (status === 'critical') {
            alerts.push({
              type: 'critical',
              meter: meterName,
              value,
              max: meterConfig.range[1],
              message: rules.guidance_templates?.[`${meterName}_critical`] ||
                `${meterName.toUpperCase()} CRITICAL (${value}/${meterConfig.range[1]})`,
              guidance,
              guidanceSource
            });
          } else if (status === 'alert') {
            alerts.push({
              type: 'alert',
              meter: meterName,
              value,
              max: meterConfig.range[1],
              message: rules.guidance_templates?.[`${meterName}_high`] ||
                `${meterName.toUpperCase()} elevated (${value}/${meterConfig.range[1]})`,
              guidance,
              guidanceSource
            });
          } else if (status === 'low') {
            alerts.push({
              type: 'low',
              meter: meterName,
              value,
              max: meterConfig.range[1],
              message: rules.guidance_templates?.[`${meterName}_low`] ||
                `${meterName.toUpperCase()} low (${value}/${meterConfig.range[1]})`,
              guidance,
              guidanceSource
            });
          }
        }
      }

      // Check beat changes
      if (state.beat && state.beat !== previousState.beat) {
        const beatConfig = rules.beats?.[state.beat];
        if (beatConfig) {
          alerts.push({
            type: 'beat',
            beat: state.beat,
            message: `Beat: ${state.beat.toUpperCase()}`,
            guidance: beatConfig.guidance || beatConfig.description
          });
        }
      }

      // Check for director's note (ephemeral)
      // Agent-specific notes are in notes/{agent_name}.txt
      const agentName = process.env.CLAUDE_AGENT_NAME;
      let directorNote = null;

      if (agentName) {
        // Agent-specific note - check for exact match or partial match
        const notesDir = path.join(stateDir, 'notes');
        console.error(`[scene-state] Looking for notes for agent: ${agentName}`);

        if (fs.existsSync(notesDir)) {
          const noteFiles = fs.readdirSync(notesDir);

          // Find matching note file (exact match, or agent name contains file prefix)
          // e.g., agent "detective_rourke" matches "rourke.txt" or "detective_rourke.txt"
          const matchingFile = noteFiles.find(file => {
            if (!file.endsWith('.txt')) return false;
            const noteName = file.replace('.txt', '').toLowerCase();
            return agentName === noteName || agentName.includes(noteName) || noteName.includes(agentName);
          });

          if (matchingFile) {
            const agentNotePath = path.join(notesDir, matchingFile);
            console.error(`[scene-state] Found matching note: ${matchingFile}`);
            directorNote = fs.readFileSync(agentNotePath, 'utf-8').trim();
            if (directorNote) {
              // Clear after reading (ephemeral)
              try {
                fs.unlinkSync(agentNotePath);
                console.error(`[scene-state] Consumed note for ${agentName}`);
              } catch {}
            }
          }
        }
      }

      // Fallback: generic director note (for backwards compatibility)
      if (!directorNote) {
        const directorNotePath = path.join(stateDir, 'director-note.txt');
        if (fs.existsSync(directorNotePath)) {
          directorNote = fs.readFileSync(directorNotePath, 'utf-8').trim();
          if (directorNote) {
            try {
              fs.unlinkSync(directorNotePath);
            } catch {}
          }
        }
      }

      // Generate output only if there's something to report
      if (alerts.length > 0 || directorNote) {
        let output = '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n';
        output += '🎬 SCENE STATE UPDATE\n';
        output += '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n';

        // Log what we're about to inject (for debugging and evaluation)
        const injectionLog = {
          timestamp: new Date().toISOString(),
          agent: agentName || 'unknown',
          alerts: alerts.map(a => ({
            type: a.type,
            meter: a.meter,
            value: a.value,
            beat: a.beat,
            guidance: a.guidance,
            guidanceSource: a.guidanceSource  // 'dynamic' (from Haiku) or 'fallback' (hardcoded)
          })),
          director_note: directorNote || null,  // Log actual content, not just boolean
          current_state: {
            beat: state.beat,
            tension: state.meters.tension,
            heat: state.meters.heat
          }
        };
        const logsDir = path.join(claudeDir, 'logs');
        fs.mkdirSync(logsDir, { recursive: true });
        fs.appendFileSync(
          path.join(logsDir, 'injections.jsonl'),
          JSON.stringify(injectionLog) + '\n',
          'utf-8'
        );
        console.error(`[scene-state] INJECTING context to ${agentName}: ${alerts.length} alerts, directorNote=${!!directorNote}`);

        // Location if set
        if (state.location) {
          output += `Location: ${state.location}\n\n`;
        }

        // Alerts
        for (const alert of alerts) {
          if (alert.type === 'critical') {
            output += `⚠️ ${alert.message}\n`;
          } else if (alert.type === 'alert') {
            output += `📊 ${alert.message}\n`;
          } else if (alert.type === 'low') {
            output += `💡 ${alert.message}\n`;
          } else if (alert.type === 'beat') {
            output += `🎭 ${alert.message}\n`;
          }

          if (alert.guidance) {
            output += `   ${alert.guidance}\n`;
          }
          output += '\n';
        }

        // Director's note
        if (directorNote) {
          output += '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n';
          output += '📢 DIRECTOR\'S NOTE (THIS TURN ONLY)\n';
          output += directorNote + '\n';
          output += '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n';
        }

        console.log(output);
      }

      // Save current state as previous for next comparison
      savePreviousState(stateDir, state);

      process.exit(0);

    } catch (error) {
      // Silent exit on errors
      process.exit(0);
    }
  });
}

/**
 * Load current scene state from files
 */
function loadState(stateDir) {
  const state = {
    meters: {},
    beat: null,
    location: null
  };

  if (!fs.existsSync(stateDir)) {
    return state;
  }

  // Load meters
  const metersDir = path.join(stateDir, 'meters');
  if (fs.existsSync(metersDir)) {
    for (const file of fs.readdirSync(metersDir)) {
      if (file.endsWith('.txt')) {
        const meterName = file.replace('.txt', '');
        const value = parseInt(fs.readFileSync(path.join(metersDir, file), 'utf-8').trim(), 10);
        if (!isNaN(value)) {
          state.meters[meterName] = value;
        }
      }
    }
  }

  // Load beat
  const beatFile = path.join(stateDir, 'beat.txt');
  if (fs.existsSync(beatFile)) {
    state.beat = fs.readFileSync(beatFile, 'utf-8').trim();
  }

  // Load location
  const locationFile = path.join(stateDir, 'location.txt');
  if (fs.existsSync(locationFile)) {
    state.location = fs.readFileSync(locationFile, 'utf-8').trim();
  }

  return state;
}

/**
 * Load previous state for comparison
 */
function loadPreviousState(stateDir) {
  const prevFile = path.join(stateDir, '.previous-state.json');
  if (fs.existsSync(prevFile)) {
    try {
      return JSON.parse(fs.readFileSync(prevFile, 'utf-8'));
    } catch {
      return { meters: {}, beat: null };
    }
  }
  return { meters: {}, beat: null };
}

/**
 * Save current state for next comparison
 */
function savePreviousState(stateDir, state) {
  fs.mkdirSync(stateDir, { recursive: true });
  const prevFile = path.join(stateDir, '.previous-state.json');
  fs.writeFileSync(prevFile, JSON.stringify(state), 'utf-8');
}

/**
 * Get the level key for a value (e.g., "1-2", "3-4")
 */
function getLevelKey(value, range) {
  // Try exact match first
  if (range) {
    // Try common range formats
    const formats = [
      `${value}`,
      `${value}-${value}`,
      `${Math.floor((value - 1) / 2) * 2 + 1}-${Math.floor((value - 1) / 2) * 2 + 2}`
    ];
    return formats.find(f => f) || String(value);
  }
  return String(value);
}

process.stdin.on('error', () => {
  process.exit(0);
});

main();
