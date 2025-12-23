#!/usr/bin/env node

/**
 * PostToolUse hook to log tool usage from agents (including Haiku analyzer)
 * Helps debug whether Haiku is using the Write tool
 */

import fs from 'fs';
import path from 'path';

function main() {
  let input = '';

  process.stdin.on('data', chunk => {
    input += chunk;
  });

  process.stdin.on('end', () => {
    try {
      const data = JSON.parse(input);
      const projectDir = process.env.CLAUDE_PROJECT_DIR;

      if (!projectDir) {
        process.exit(0);
      }

      const logDir = path.join(projectDir, '.claude', 'logs');
      fs.mkdirSync(logDir, { recursive: true });

      const logEntry = {
        timestamp: new Date().toISOString(),
        agent: process.env.CLAUDE_AGENT_NAME || 'unknown',
        session_id: data.session_id,
        tool_name: data.tool_name,
        tool_input: data.tool_input,
        tool_response_preview: typeof data.tool_response === 'string'
          ? data.tool_response.slice(0, 200)
          : JSON.stringify(data.tool_response).slice(0, 200),
      };

      // Log to JSONL
      const logPath = path.join(logDir, 'tool-usage.jsonl');
      fs.appendFileSync(logPath, JSON.stringify(logEntry) + '\n', 'utf8');

      // Also log to stderr for immediate visibility
      console.error(`[post-tool-use] ${logEntry.agent}: ${data.tool_name}`);

      process.exit(0);

    } catch (error) {
      console.error(`[post-tool-use] Error: ${error.message}`);
      process.exit(0);
    }
  });
}

process.stdin.on('error', () => process.exit(0));

main();
