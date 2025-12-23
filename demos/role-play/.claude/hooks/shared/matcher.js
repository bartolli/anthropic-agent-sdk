/**
 * Shared matcher module for keyword and intent pattern matching.
 * Reusable across skill activation, scene state, and other hooks.
 */

/**
 * Match text against keyword list (case-insensitive)
 * @param {string} text - Text to search in
 * @param {string[]} keywords - Keywords to match
 * @returns {string[]} - Matched keywords
 */
export function matchKeywords(text, keywords) {
  if (!keywords || !Array.isArray(keywords)) return [];
  const lowerText = text.toLowerCase();
  return keywords.filter(kw => lowerText.includes(kw.toLowerCase()));
}

/**
 * Match text against intent patterns (regex, case-insensitive)
 * @param {string} text - Text to search in
 * @param {string[]} patterns - Regex patterns to match
 * @returns {string[]} - Matched patterns
 */
export function matchIntentPatterns(text, patterns) {
  if (!patterns || !Array.isArray(patterns)) return [];
  return patterns.filter(pattern => {
    try {
      const regex = new RegExp(pattern, 'i');
      return regex.test(text);
    } catch {
      return false;
    }
  });
}

/**
 * Check if text matches any triggers (keywords or intent patterns)
 * @param {string} text - Text to search in
 * @param {{ keywords?: string[], intentPatterns?: string[] }} triggers - Trigger config
 * @returns {{ matched: boolean, keywords: string[], patterns: string[] }}
 */
export function matchTriggers(text, triggers) {
  if (!triggers) return { matched: false, keywords: [], patterns: [] };

  const keywords = matchKeywords(text, triggers.keywords);
  const patterns = matchIntentPatterns(text, triggers.intentPatterns);

  return {
    matched: keywords.length > 0 || patterns.length > 0,
    keywords,
    patterns
  };
}

/**
 * Calculate meter delta based on modifier keywords
 * @param {string} text - Text to analyze
 * @param {{ increase?: string[], decrease?: string[] }} modifiers - Modifier keywords
 * @returns {number} - Net change (positive = increase, negative = decrease)
 */
export function calculateMeterDelta(text, modifiers) {
  if (!modifiers) return 0;

  const lowerText = text.toLowerCase();
  let delta = 0;

  if (modifiers.increase) {
    for (const word of modifiers.increase) {
      if (lowerText.includes(word.toLowerCase())) delta += 1;
    }
  }

  if (modifiers.decrease) {
    for (const word of modifiers.decrease) {
      if (lowerText.includes(word.toLowerCase())) delta -= 1;
    }
  }

  return delta;
}

/**
 * Clamp a value to a range
 * @param {number} value - Value to clamp
 * @param {number} min - Minimum
 * @param {number} max - Maximum
 * @returns {number}
 */
export function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

/**
 * Get threshold status for a meter value
 * @param {number} value - Current value
 * @param {{ alert_above?: number, critical_above?: number, low_below?: number }} thresholds
 * @returns {'critical' | 'alert' | 'low' | 'normal'}
 */
export function getThresholdStatus(value, thresholds) {
  if (!thresholds) return 'normal';

  if (thresholds.critical_above !== undefined && value > thresholds.critical_above) {
    return 'critical';
  }
  if (thresholds.alert_above !== undefined && value > thresholds.alert_above) {
    return 'alert';
  }
  if (thresholds.low_below !== undefined && value < thresholds.low_below) {
    return 'low';
  }
  return 'normal';
}
