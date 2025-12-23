#!/usr/bin/env node

/**
 * State Updater Hook (PostToolUse)
 *
 * Analyzes dialogue output and updates scene state:
 * - Updates meters based on modifier keywords
 * - Detects beat transitions based on triggers
 * - Logs dialogue for analysis
 *
 * Uses CLAUDE_PROJECT_DIR for path resolution.
 * Uses shared/matcher.js for trigger matching.
 */

import fs from 'fs';
import path from 'path';
import { matchTriggers, calculateMeterDelta, clamp } from './shared/matcher.js';

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
      console.error(`[state-updater] CLAUDE_PROJECT_DIR=${projectDir}`);
      console.error(`[state-updater] tool_name=${data.tool_name}`);

      if (!projectDir) {
        console.error('[state-updater] No CLAUDE_PROJECT_DIR, exiting');
        process.exit(0);
      }

      const claudeDir = path.join(projectDir, '.claude');
      const stateDir = path.join(claudeDir, 'scene-state');
      const metersDir = path.join(stateDir, 'meters');
      const logsDir = path.join(claudeDir, 'logs');
      const rulesPath = path.join(claudeDir, 'scene-rules.json');

      // Ensure directories exist
      fs.mkdirSync(metersDir, { recursive: true });
      fs.mkdirSync(logsDir, { recursive: true });

      // Load scene rules
      if (!fs.existsSync(rulesPath)) {
        process.exit(0);
      }

      const rules = JSON.parse(fs.readFileSync(rulesPath, 'utf-8'));

      // Extract text from tool response
      const responseText = extractText(data.tool_response);

      if (!responseText) {
        process.exit(0);
      }

      // Log dialogue
      const logEntry = {
        timestamp: new Date().toISOString(),
        session_id: data.session_id,
        tool_name: data.tool_name,
        text_preview: responseText.slice(0, 300)
      };
      fs.appendFileSync(
        path.join(logsDir, 'dialogue.jsonl'),
        JSON.stringify(logEntry) + '\n',
        'utf-8'
      );

      // Update meters
      for (const [meterName, meterConfig] of Object.entries(rules.meters || {})) {
        const meterFile = path.join(metersDir, `${meterName}.txt`);

        // Read current value
        let value = meterConfig.default;
        if (fs.existsSync(meterFile)) {
          value = parseInt(fs.readFileSync(meterFile, 'utf-8').trim(), 10);
          if (isNaN(value)) value = meterConfig.default;
        }

        // Calculate delta
        const delta = calculateMeterDelta(responseText, meterConfig.modifiers);

        if (delta !== 0) {
          const [min, max] = meterConfig.range;
          value = clamp(value + delta, min, max);
          fs.writeFileSync(meterFile, String(value), 'utf-8');

          // Log meter change
          fs.appendFileSync(
            path.join(logsDir, 'meter-changes.jsonl'),
            JSON.stringify({
              timestamp: new Date().toISOString(),
              meter: meterName,
              delta,
              new_value: value
            }) + '\n',
            'utf-8'
          );
        }
      }

      // Check for beat transitions
      const beatFile = path.join(stateDir, 'beat.txt');
      let currentBeat = 'exposition';
      if (fs.existsSync(beatFile)) {
        currentBeat = fs.readFileSync(beatFile, 'utf-8').trim() || 'exposition';
      }

      for (const [beatName, beatConfig] of Object.entries(rules.beats || {})) {
        if (beatName === currentBeat) continue;

        const { matched } = matchTriggers(responseText, beatConfig.triggers);
        if (matched) {
          // Beat transition detected
          fs.writeFileSync(beatFile, beatName, 'utf-8');

          // Apply beat effects to meters
          if (beatConfig.effects) {
            for (const [meterName, delta] of Object.entries(beatConfig.effects)) {
              const meterConfig = rules.meters[meterName];
              if (!meterConfig) continue;

              const meterFile = path.join(metersDir, `${meterName}.txt`);
              let value = meterConfig.default;
              if (fs.existsSync(meterFile)) {
                value = parseInt(fs.readFileSync(meterFile, 'utf-8').trim(), 10);
                if (isNaN(value)) value = meterConfig.default;
              }

              const [min, max] = meterConfig.range;
              value = clamp(value + delta, min, max);
              fs.writeFileSync(meterFile, String(value), 'utf-8');
            }
          }

          // Log beat transition
          fs.appendFileSync(
            path.join(logsDir, 'beat-transitions.jsonl'),
            JSON.stringify({
              timestamp: new Date().toISOString(),
              from: currentBeat,
              to: beatName,
              trigger: 'keyword_match'
            }) + '\n',
            'utf-8'
          );

          break; // Only one beat transition per turn
        }
      }

      process.exit(0);

    } catch (error) {
      // Silent exit on errors
      process.exit(0);
    }
  });
}

/**
 * Extract readable text from tool response
 */
function extractText(response) {
  if (!response) return '';

  if (typeof response === 'string') {
    return response;
  }

  // Handle common response formats
  if (response.content) {
    return typeof response.content === 'string'
      ? response.content
      : JSON.stringify(response.content);
  }

  if (response.text) {
    return response.text;
  }

  if (response.message) {
    return typeof response.message === 'string'
      ? response.message
      : JSON.stringify(response.message);
  }

  // Fallback: stringify the whole thing
  return JSON.stringify(response);
}

process.stdin.on('error', () => {
  process.exit(0);
});

main();
