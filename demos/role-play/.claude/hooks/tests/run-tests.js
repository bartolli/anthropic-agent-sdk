#!/usr/bin/env node

/**
 * Test suite for role-play scene hooks
 *
 * Tests:
 * 1. shared/matcher.js - keyword and pattern matching
 * 2. scene-state-hook.js - UserPromptSubmit context injection
 * 3. state-updater-hook.js - PostToolUse state updates
 *
 * Run: node tests/run-tests.js
 */

import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const hooksDir = path.resolve(__dirname, '..');
const projectDir = path.resolve(hooksDir, '../..');

// Test results
let passed = 0;
let failed = 0;

function test(name, fn) {
  try {
    fn();
    console.log(`✓ ${name}`);
    passed++;
  } catch (error) {
    console.log(`✗ ${name}`);
    console.log(`  Error: ${error.message}`);
    failed++;
  }
}

function assertEqual(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(`${message}: expected "${expected}", got "${actual}"`);
  }
}

function assertTrue(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

async function runHook(hookPath, input, env = {}) {
  return new Promise((resolve, reject) => {
    const proc = spawn('node', [hookPath], {
      cwd: hooksDir,
      env: { ...process.env, ...env },
      stdio: ['pipe', 'pipe', 'pipe']
    });

    let stdout = '';
    let stderr = '';

    proc.stdout.on('data', data => { stdout += data; });
    proc.stderr.on('data', data => { stderr += data; });

    proc.on('close', code => {
      resolve({ code, stdout, stderr });
    });

    proc.on('error', reject);

    proc.stdin.write(JSON.stringify(input));
    proc.stdin.end();
  });
}

async function runMatcherTests() {
  console.log('\n━━━ Matcher Module Tests ━━━\n');

  // Dynamic import
  const matcher = await import('../shared/matcher.js');

  test('matchKeywords: finds keywords', () => {
    const result = matcher.matchKeywords('I confess the truth', ['confess', 'truth']);
    assertTrue(result.includes('confess'), 'should find "confess"');
    assertTrue(result.includes('truth'), 'should find "truth"');
  });

  test('matchKeywords: case insensitive', () => {
    const result = matcher.matchKeywords('I CONFESS', ['confess']);
    assertTrue(result.includes('confess'), 'should match case-insensitive');
  });

  test('matchKeywords: no match', () => {
    const result = matcher.matchKeywords('Hello world', ['confess']);
    assertEqual(result.length, 0, 'should have no matches');
  });

  test('matchIntentPatterns: regex match', () => {
    const result = matcher.matchIntentPatterns('I finally understand', ['(finally|now).*understand']);
    assertEqual(result.length, 1, 'should match pattern');
  });

  test('matchTriggers: combined match', () => {
    const result = matcher.matchTriggers('I confess the truth', {
      keywords: ['confess'],
      intentPatterns: ['the truth']
    });
    assertTrue(result.matched, 'should match');
    assertTrue(result.keywords.includes('confess'), 'should have keyword match');
  });

  test('calculateMeterDelta: increase', () => {
    const delta = matcher.calculateMeterDelta('I feel fear and danger', {
      increase: ['fear', 'danger'],
      decrease: ['calm']
    });
    assertEqual(delta, 2, 'should increase by 2');
  });

  test('calculateMeterDelta: mixed', () => {
    const delta = matcher.calculateMeterDelta('I feel fear but also peace', {
      increase: ['fear'],
      decrease: ['peace']
    });
    assertEqual(delta, 0, 'should cancel out');
  });

  test('clamp: within range', () => {
    assertEqual(matcher.clamp(5, 1, 10), 5, 'should stay 5');
  });

  test('clamp: below min', () => {
    assertEqual(matcher.clamp(-1, 1, 10), 1, 'should clamp to 1');
  });

  test('clamp: above max', () => {
    assertEqual(matcher.clamp(15, 1, 10), 10, 'should clamp to 10');
  });

  test('getThresholdStatus: critical', () => {
    const status = matcher.getThresholdStatus(9, { critical_above: 8 });
    assertEqual(status, 'critical', 'should be critical');
  });

  test('getThresholdStatus: normal', () => {
    const status = matcher.getThresholdStatus(5, { critical_above: 8, alert_above: 6 });
    assertEqual(status, 'normal', 'should be normal');
  });
}

async function runSceneStateHookTests() {
  console.log('\n━━━ Scene State Hook Tests ━━━\n');

  const hookPath = path.join(hooksDir, 'scene-state-hook.js');
  const stateDir = path.join(projectDir, '.claude', 'scene-state');
  const metersDir = path.join(stateDir, 'meters');

  // Setup: create test state
  fs.mkdirSync(metersDir, { recursive: true });

  test('scene-state-hook: exits 0 without CLAUDE_PROJECT_DIR', async () => {
    const result = await runHook(hookPath, { prompt: 'test' }, {});
    assertEqual(result.code, 0, 'should exit 0');
  });

  test('scene-state-hook: exits 0 with empty state (no output)', async () => {
    // Clean state
    fs.rmSync(stateDir, { recursive: true, force: true });

    const result = await runHook(hookPath, { prompt: 'test' }, {
      CLAUDE_PROJECT_DIR: projectDir
    });
    assertEqual(result.code, 0, 'should exit 0');
    assertEqual(result.stdout.trim(), '', 'should have no output when no threshold crossed');
  });

  test('scene-state-hook: outputs on threshold crossing', async () => {
    // Setup: create state with high tension
    fs.mkdirSync(metersDir, { recursive: true });
    fs.writeFileSync(path.join(metersDir, 'tension.txt'), '9');
    fs.writeFileSync(path.join(stateDir, 'beat.txt'), 'confrontation');

    // Clear previous state to trigger threshold detection
    const prevStateFile = path.join(stateDir, '.previous-state.json');
    if (fs.existsSync(prevStateFile)) {
      fs.unlinkSync(prevStateFile);
    }

    const result = await runHook(hookPath, { prompt: 'test' }, {
      CLAUDE_PROJECT_DIR: projectDir
    });

    assertEqual(result.code, 0, 'should exit 0');
    assertTrue(result.stdout.includes('SCENE STATE'), 'should include scene state header');
    assertTrue(result.stdout.includes('CRITICAL') || result.stdout.includes('tension'),
      'should mention tension');
  });

  test('scene-state-hook: handles director note (ephemeral)', async () => {
    // Setup: create director note
    fs.mkdirSync(stateDir, { recursive: true });
    fs.writeFileSync(path.join(stateDir, 'director-note.txt'), 'Increase the drama!');

    const result = await runHook(hookPath, { prompt: 'test' }, {
      CLAUDE_PROJECT_DIR: projectDir
    });

    assertEqual(result.code, 0, 'should exit 0');
    assertTrue(result.stdout.includes('DIRECTOR'), 'should include director note');
    assertTrue(result.stdout.includes('Increase the drama'), 'should include note content');

    // Verify note was deleted (ephemeral)
    assertTrue(!fs.existsSync(path.join(stateDir, 'director-note.txt')),
      'director note should be deleted after read');
  });
}

async function runStateUpdaterHookTests() {
  console.log('\n━━━ State Updater Hook Tests ━━━\n');

  const hookPath = path.join(hooksDir, 'state-updater-hook.js');
  const stateDir = path.join(projectDir, '.claude', 'scene-state');
  const metersDir = path.join(stateDir, 'meters');
  const logsDir = path.join(projectDir, '.claude', 'logs');

  // Clean up
  fs.rmSync(stateDir, { recursive: true, force: true });
  fs.rmSync(logsDir, { recursive: true, force: true });
  fs.mkdirSync(metersDir, { recursive: true });

  test('state-updater-hook: exits 0 without CLAUDE_PROJECT_DIR', async () => {
    const result = await runHook(hookPath, { tool_response: { content: 'test' } }, {});
    assertEqual(result.code, 0, 'should exit 0');
  });

  test('state-updater-hook: updates tension on keywords', async () => {
    // Set initial tension
    fs.writeFileSync(path.join(metersDir, 'tension.txt'), '5');

    const input = {
      session_id: 'test',
      tool_name: 'Write',
      tool_response: {
        content: 'I feel fear and danger approaching'
      }
    };

    const result = await runHook(hookPath, input, {
      CLAUDE_PROJECT_DIR: projectDir
    });

    assertEqual(result.code, 0, 'should exit 0');

    // Check tension increased
    const tension = parseInt(fs.readFileSync(path.join(metersDir, 'tension.txt'), 'utf-8'));
    assertTrue(tension > 5, `tension should increase from 5, got ${tension}`);
  });

  test('state-updater-hook: detects beat transition', async () => {
    // Set initial beat
    fs.writeFileSync(path.join(stateDir, 'beat.txt'), 'exposition');

    const input = {
      session_id: 'test',
      tool_name: 'Write',
      tool_response: {
        content: 'I confess everything. The secret truth must be revealed.'
      }
    };

    const result = await runHook(hookPath, input, {
      CLAUDE_PROJECT_DIR: projectDir
    });

    assertEqual(result.code, 0, 'should exit 0');

    // Check beat changed
    const beat = fs.readFileSync(path.join(stateDir, 'beat.txt'), 'utf-8').trim();
    assertEqual(beat, 'revelation', `beat should be "revelation", got "${beat}"`);
  });

  test('state-updater-hook: logs dialogue', async () => {
    const input = {
      session_id: 'test-log',
      tool_name: 'Write',
      tool_response: {
        content: 'Test dialogue content'
      }
    };

    const result = await runHook(hookPath, input, {
      CLAUDE_PROJECT_DIR: projectDir
    });

    assertEqual(result.code, 0, 'should exit 0');

    // Check log file exists
    const logPath = path.join(logsDir, 'dialogue.jsonl');
    assertTrue(fs.existsSync(logPath), 'dialogue log should exist');

    const logContent = fs.readFileSync(logPath, 'utf-8');
    assertTrue(logContent.includes('test-log'), 'log should contain session_id');
  });
}

async function runStopHookTests() {
  console.log('\n━━━ State Updater Stop Hook Tests ━━━\n');

  const hookPath = path.join(hooksDir, 'state-updater-stop-hook.js');
  const stateDir = path.join(projectDir, '.claude', 'scene-state');
  const metersDir = path.join(stateDir, 'meters');
  const logsDir = path.join(projectDir, '.claude', 'logs');
  const testTranscriptPath = path.join(logsDir, 'test-transcript.jsonl');

  // Clean up
  fs.rmSync(stateDir, { recursive: true, force: true });
  fs.rmSync(logsDir, { recursive: true, force: true });
  fs.mkdirSync(metersDir, { recursive: true });
  fs.mkdirSync(logsDir, { recursive: true });

  // Create test transcript
  function createTranscript(assistantText) {
    const lines = [
      JSON.stringify({ type: 'user', sessionId: 'test-session', message: { content: 'Hello' } }),
      JSON.stringify({
        type: 'assistant',
        sessionId: 'test-session',
        message: {
          content: [{ type: 'text', text: assistantText }]
        }
      })
    ];
    fs.writeFileSync(testTranscriptPath, lines.join('\n') + '\n');
  }

  test('stop-hook: exits 0 without CLAUDE_PROJECT_DIR', async () => {
    const result = await runHook(hookPath, { transcript_path: testTranscriptPath }, {});
    assertEqual(result.code, 0, 'should exit 0');
  });

  test('stop-hook: exits 0 without transcript_path', async () => {
    const result = await runHook(hookPath, {}, {
      CLAUDE_PROJECT_DIR: projectDir
    });
    assertEqual(result.code, 0, 'should exit 0');
  });

  test('stop-hook: parses transcript and extracts dialogue', async () => {
    createTranscript('This is a test dialogue');

    const result = await runHook(hookPath, { transcript_path: testTranscriptPath }, {
      CLAUDE_PROJECT_DIR: projectDir
    });

    assertEqual(result.code, 0, 'should exit 0');
    assertTrue(result.stderr.includes('Dialogue preview'), 'should log dialogue preview');
    assertTrue(result.stderr.includes('This is a test'), 'should include dialogue text');
  });

  test('stop-hook: updates tension on fear/danger keywords', async () => {
    // Reset tension
    fs.writeFileSync(path.join(metersDir, 'tension.txt'), '5');

    createTranscript('I feel the threat of danger approaching, fear grips me');

    const result = await runHook(hookPath, { transcript_path: testTranscriptPath }, {
      CLAUDE_PROJECT_DIR: projectDir
    });

    assertEqual(result.code, 0, 'should exit 0');

    const tension = parseInt(fs.readFileSync(path.join(metersDir, 'tension.txt'), 'utf-8'));
    assertTrue(tension > 5, `tension should increase from 5, got ${tension}`);
    assertTrue(result.stderr.includes('tension:'), 'should log tension change');
  });

  test('stop-hook: decreases tension on calm keywords', async () => {
    // Set high tension
    fs.writeFileSync(path.join(metersDir, 'tension.txt'), '7');

    createTranscript('Everything is calm now, I feel trust and peace');

    const result = await runHook(hookPath, { transcript_path: testTranscriptPath }, {
      CLAUDE_PROJECT_DIR: projectDir
    });

    assertEqual(result.code, 0, 'should exit 0');

    const tension = parseInt(fs.readFileSync(path.join(metersDir, 'tension.txt'), 'utf-8'));
    assertTrue(tension < 7, `tension should decrease from 7, got ${tension}`);
  });

  test('stop-hook: detects beat transition to revelation', async () => {
    fs.writeFileSync(path.join(stateDir, 'beat.txt'), 'exposition');

    createTranscript('I must confess the secret truth. This revelation changes everything.');

    const result = await runHook(hookPath, { transcript_path: testTranscriptPath }, {
      CLAUDE_PROJECT_DIR: projectDir
    });

    assertEqual(result.code, 0, 'should exit 0');

    const beat = fs.readFileSync(path.join(stateDir, 'beat.txt'), 'utf-8').trim();
    assertEqual(beat, 'revelation', `beat should be "revelation", got "${beat}"`);
    assertTrue(result.stderr.includes('Beat transition'), 'should log beat transition');
  });

  test('stop-hook: updates heat meter on intimate keywords', async () => {
    fs.writeFileSync(path.join(metersDir, 'heat.txt'), '1');

    createTranscript('She moved close, their lips almost touching');

    const result = await runHook(hookPath, { transcript_path: testTranscriptPath }, {
      CLAUDE_PROJECT_DIR: projectDir
    });

    assertEqual(result.code, 0, 'should exit 0');

    const heat = parseInt(fs.readFileSync(path.join(metersDir, 'heat.txt'), 'utf-8'));
    assertTrue(heat > 1, `heat should increase from 1, got ${heat}`);
  });

  test('stop-hook: logs dialogue to jsonl', async () => {
    // Clear dialogue log
    const dialogueLog = path.join(logsDir, 'dialogue.jsonl');
    if (fs.existsSync(dialogueLog)) {
      fs.unlinkSync(dialogueLog);
    }

    createTranscript('Logged dialogue content here');

    const result = await runHook(hookPath, { transcript_path: testTranscriptPath }, {
      CLAUDE_PROJECT_DIR: projectDir
    });

    assertEqual(result.code, 0, 'should exit 0');
    assertTrue(fs.existsSync(dialogueLog), 'dialogue log should exist');

    const logContent = fs.readFileSync(dialogueLog, 'utf-8');
    assertTrue(logContent.includes('test-session'), 'should log session_id');
    assertTrue(logContent.includes('Logged dialogue'), 'should log text preview');
  });

  test('stop-hook: logs meter changes to jsonl', async () => {
    fs.writeFileSync(path.join(metersDir, 'tension.txt'), '5');

    // Clear meter log
    const meterLog = path.join(logsDir, 'meter-changes.jsonl');
    if (fs.existsSync(meterLog)) {
      fs.unlinkSync(meterLog);
    }

    createTranscript('Danger! Threat! Fear!');

    const result = await runHook(hookPath, { transcript_path: testTranscriptPath }, {
      CLAUDE_PROJECT_DIR: projectDir
    });

    assertEqual(result.code, 0, 'should exit 0');
    assertTrue(fs.existsSync(meterLog), 'meter changes log should exist');

    const logContent = fs.readFileSync(meterLog, 'utf-8');
    assertTrue(logContent.includes('tension'), 'should log meter name');
    assertTrue(logContent.includes('delta'), 'should log delta');
  });

  // Cleanup test transcript
  if (fs.existsSync(testTranscriptPath)) {
    fs.unlinkSync(testTranscriptPath);
  }
}

async function main() {
  console.log('═══════════════════════════════════════');
  console.log('  Role-Play Scene Hooks Test Suite');
  console.log('═══════════════════════════════════════');

  await runMatcherTests();
  await runSceneStateHookTests();
  await runStateUpdaterHookTests();
  await runStopHookTests();

  console.log('\n═══════════════════════════════════════');
  console.log(`  Results: ${passed} passed, ${failed} failed`);
  console.log('═══════════════════════════════════════\n');

  process.exit(failed > 0 ? 1 : 0);
}

main().catch(err => {
  console.error('Test suite error:', err);
  process.exit(1);
});
