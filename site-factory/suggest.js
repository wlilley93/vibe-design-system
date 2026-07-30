'use strict';

/*
 * suggest.js: propose a value for every field in config-schema.js from a short brief.
 *
 * This is a rule-based heuristic, not a live model call — there is no API key wired
 * into this script and it does not pretend otherwise. Every rule below is legible and
 * inspectable; nothing is a black box. It exists so the wizard can hand the user a
 * fully-filled ~35-field config to review and edit, instead of 35 blank prompts.
 *
 * A future version could replace SUGGEST_RULES's pattern matching with an actual
 * Claude API call over the same brief -> same schema -> same shape contract; the
 * wizard doesn't care which one filled the object, only that every field arrives.
 */

const { LAYERS } = require('./config-schema.js');

const PACKS = {
  placeholder: require('./tokens/placeholder.json'),
  geist: require('./tokens/geist.json'),
  balmoral: require('./tokens/balmoral.json'),
  jellytot: require('./tokens/jellytot.json'),
};

const STRATEGY_PLAYS = {
  'Onboarding': ['Empty States', 'Setup Defaults', 'Progressive Disclosure', 'Time to Value', 'Success Moments', 'JTBD Copywriting', 'Pattern Alignment', 'Sandbox Experience', 'Personalisation', 'Momentum Bias', 'Micro Interactions'],
  'Trust Building': ['Fail Safe', 'Pattern Alignment', 'JTBD Copywriting', 'Loading Feedback', 'Perceived Effort Delay'],
  'Monetisation': ['The Paywall', 'Intentional Friction', 'Limited Offer', 'Perceived Effort Delay', 'Success Moments', 'Time to Value', 'Spark Curiosity'],
  'Conversion Optimisation': ['The Paywall', 'Limited Offer', 'Intentional Friction', 'Success Moments', 'Spark Curiosity', 'Momentum Bias'],
  'Activation': ['Time to Value', 'Success Moments', 'Setup Defaults', 'Empty States', 'Discovery', 'Personalisation', 'Micro Interactions', 'Momentum Bias'],
  'Experience Refinement': ['Micro Interactions', 'Small Quirk', 'Loading Feedback', 'Success Moments', 'Perceived Effort Delay'],
  'Retention': ['Gamified Progress', 'System Widget', 'Value Replay', 'Effort Moat', 'Personalisation', 'Variable Reward', 'Commitment'],
  'Intent Shaping': ['Investment', 'Personalisation', 'Setup Defaults', 'Progressive Disclosure', 'Commitment'],
  'Growth & Viral': ['Referral', 'Shareability', 'Contact Bridge', 'Deep-link', 'Success Moments'],
  'Habit Formation': ['Commitment', 'Gamified Progress', 'System Widget', 'Variable Reward', 'Value Replay', 'Investment', 'Personalisation'],
  'Engagement': ['Micro Interactions', 'Gamified Progress', 'Variable Reward', 'Discovery', 'Personalisation', 'Intent Mirroring'],
  'Premium Positioning': ['Perceived Effort Delay', 'Small Quirk', 'Micro Interactions', 'Intentional Friction', 'Loading Feedback'],
};

const KEYWORD_PACKS = [
  { re: /\b(legal|law|trust|advisory|estate|ownership|structur|govern|compliance)\b/i, pack: 'balmoral', register: 'A-institutional-authority', radius: 'sharp-0' },
  { re: /\b(playful|fun|kids|game|creative|community|hobby)\b/i, pack: 'jellytot', register: 'C-voice-with-a-face', radius: 'round-16' },
  { re: /\b(saas|dashboard|platform|tool|workflow|api|developer|analytics)\b/i, pack: 'geist', register: 'A-institutional-authority', radius: 'soft-6' },
];

function pickPack(text) {
  for (const rule of KEYWORD_PACKS) if (rule.re.test(text)) return rule;
  return { pack: 'geist', register: 'A-institutional-authority', radius: 'soft-6' };
}

function suggest(brief) {
  const { name = 'Your Project', tagline = '', category = 'marketing-site', description = '' } = brief;
  const text = `${name} ${tagline} ${description}`;
  const matched = pickPack(text);
  const tokens = PACKS[matched.pack];
  const isSaas = category === 'saas-app' || category === 'hybrid';

  const strategies = isSaas
    ? ['Onboarding', 'Activation', 'Retention']
    : ['Trust Building', 'Conversion Optimisation'];
  const plays = [...new Set(strategies.flatMap((s) => STRATEGY_PLAYS[s] || []))].slice(0, 10);

  const out = {
    identity: { name, tagline, category, description },
    voice: {
      copyRegister: matched.register,
      readingLevel: /\b(technical|developer|api|engineer)\b/i.test(text) ? 'technical' : 'plain',
      ctaStyle: matched.register === 'A-institutional-authority' ? 'fact-stated' : 'verb-led',
    },
    palette: {
      basePack: matched.pack,
      groundColor: tokens.colors.bg,
      surfaceColor: tokens.colors.surface,
      inkColor: tokens.colors.ink,
      accentColor: tokens.colors.accent,
      accentInkColor: tokens.colors.accentInk,
      borderColor: tokens.colors.border,
    },
    typography: {
      displayFont: tokens.font.family,
      monoFont: tokens.font.mono,
      pairingStyle: 'single-family',
      typeScale: isSaas ? 'compact' : 'comfortable',
    },
    spacing: {
      spaceUnit: tokens.space.unit,
      density: isSaas ? 'compact' : 'comfortable',
      cornerRadius: matched.radius,
      borderWeight: 'hairline',
      elevation: isSaas ? 'soft-shadow' : 'flat',
    },
    imagery: {
      imageTreatment: matched.pack === 'balmoral' ? 'line-art' : 'photography',
      iconStyle: 'outline',
      artDirectionNote: matched.pack === 'balmoral' ? 'see ~/Projects/Balmoral/art-bank (5 lanes, pick one per use)' : '',
    },
    motion: {
      motionIntensity: isSaas ? 'subtle' : 'subtle',
      transitionStyle: 'fade',
    },
    strategy: {
      productStrategies: strategies,
      modularPlays: plays,
      sitemap: isSaas
        ? ['nav-1', 'sidebar-2', 'masterdetail-2']
        : ['nav-1', 'hero-1', 'features-1', 'pricing-1', 'testimonials-1', 'faq-1', 'cta-1', 'footer-a'],
    },
    componentStyle: {
      buttonShape: matched.radius === 'sharp-0' ? 'square' : matched.radius === 'round-16' ? 'pill' : 'rounded',
      tableDensity: isSaas ? 'compact' : 'comfortable',
      navigationPattern: isSaas ? 'sidebar' : 'top-nav',
      statusBadgeStyle: 'pill',
    },
  };

  return out;
}

module.exports = { suggest, STRATEGY_PLAYS };
