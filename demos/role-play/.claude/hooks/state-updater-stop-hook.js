#!/usr/bin/env node

/**
 * State Updater Hook (Stop)
 *
 * Fires at the end of each agent turn. Parses the transcript
 * to extract the latest dialogue and log it for analysis.
 *
 * NOTE: Meter updates and beat transitions are now handled by
 * the Haiku analyzer (spawned by orchestrator after each turn).
 * This hook now only handles dialogue logging.
 *
 * Uses CLAUDE_PROJECT_DIR for path resolution.
 */

import fs from 'fs';
import path from 'path';
import readline from 'readline';
// Note: matchTriggers, calculateMeterDelta no longer used - Haiku handles semantic analysis

/**
 * Extract the latest assistant text from the transcript
 */
async function extractLatestDialogue(transcriptPath) {
  const fileStream = fs.createReadStream(transcriptPath);
  const rl = readline.createInterface({
    input: fileStream,
    crlfDelay: Infinity,
  });

  let latestText = '';
  let sessionId = null;

  for await (const line of rl) {
    try {
      const entry = JSON.parse(line);

      if (!sessionId && entry.sessionId) {
        sessionId = entry.sessionId;
      }

      // Look for assistant messages with text content
      if (entry.type === 'assistant') {
        const content = entry.message?.content;
        if (Array.isArray(content)) {
          for (const item of content) {
            if (item.type === 'text' && item.text) {
              latestText = item.text;
            }
          }
        }
      }
    } catch {
      // Skip malformed lines
    }
  }

  return { text: latestText, sessionId };
}

async function main() {
  let input = '';

  for await (const chunk of process.stdin) {
    input += chunk;
  }

  try {
    const data = JSON.parse(input);
    const projectDir = process.env.CLAUDE_PROJECT_DIR;

    // Debug logging
    console.error(`[state-updater-stop] CLAUDE_PROJECT_DIR=${projectDir}`);
    console.error(`[state-updater-stop] transcript_path=${data.transcript_path}`);

    if (!projectDir) {
      console.error('[state-updater-stop] No CLAUDE_PROJECT_DIR, exiting');
      process.exit(0);
    }

    const transcriptPath = data.transcript_path;
    if (!transcriptPath || !fs.existsSync(transcriptPath)) {
      console.error(`[state-updater-stop] Transcript not found: ${transcriptPath}`);
      process.exit(0);
    }

    const claudeDir = path.join(projectDir, '.claude');
    const logsDir = path.join(claudeDir, 'logs');

    // Ensure logs directory exists
    fs.mkdirSync(logsDir, { recursive: true });

    // Extract latest dialogue from transcript
    const { text: responseText, sessionId } = await extractLatestDialogue(transcriptPath);

    if (!responseText) {
      console.error('[state-updater-stop] No dialogue found in transcript');
      process.exit(0);
    }

    console.error(`[state-updater-stop] Dialogue preview: ${responseText.slice(0, 80)}...`);

    // Log dialogue (keep this - useful for debugging and analysis)
    const logEntry = {
      timestamp: new Date().toISOString(),
      session_id: sessionId,
      agent: process.env.CLAUDE_AGENT_NAME || 'unknown',
      text_preview: responseText.slice(0, 300)
    };
    fs.appendFileSync(
      path.join(logsDir, 'dialogue.jsonl'),
      JSON.stringify(logEntry) + '\n',
      'utf-8'
    );

    console.error(`[state-updater-stop] Logged dialogue for ${logEntry.agent}`);

    // NOTE: Meter updates and beat transitions are now handled by Haiku analyzer
    // The orchestrator spawns Haiku after each agent turn for semantic analysis.
    // Keeping this hook for dialogue logging only.

    process.exit(0);

  } catch (error) {
    console.error(`[state-updater-stop] Error: ${error.message}`);
    process.exit(0);
  }
}

main();
