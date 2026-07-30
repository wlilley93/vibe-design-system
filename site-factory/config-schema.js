'use strict';

/*
 * config-schema.js: every layer of a site-factory project, as one ordered list.
 *
 * This is not an invented taxonomy. Each layer either comes from something already
 * built in this session or from real, already-decided research sitting in other
 * projects on this machine:
 *
 *   - Palette / Typography / Interior / Pattern-Abstract as the top-level split
 *     is Jellytot's own four-layer brand model (~/Projects/Jellytot/docs/
 *     HANDOVER-brand-2026-07-28.md, "Will's four-layer proposal, restructured").
 *   - Voice/copyRegister is Balmoral's own research: 13 real competitor sites sorted
 *     into 3 registers (brand-reference-teardown-2026-07-28.md, "05 - Copy and voice").
 *     Register B ("the interchangeable middle") is excluded as a choice on purpose —
 *     that doc names it as the failure mode to avoid, not a style.
 *   - productStrategies / modularPlays are the 12 Product Strategies and 33 Modular
 *     Plays extracted from TheProductDesignPlaybook.pdf into the VDS Site Builder
 *     Figma file (page "Playbook", root 4:3) earlier this session. Real names, real
 *     one-liners, nothing invented.
 *   - componentStyle fields map onto the SaaS component taxonomy already cataloged
 *     from Opbox's COMPONENT_INVENTORY.md (buttonShape -> Gated Action Button,
 *     statusBadgeStyle -> StatusBadge Style=Pill/Style=Dot, both already built).
 *   - basePack options are the 4 token packs that actually exist on disk in tokens/.
 *
 * Every field is EDITABLE. suggest.js proposes a value for each one from a short
 * brief (name/tagline/category/description); the wizard shows every suggestion and
 * lets the user override any of them before compiling.
 */

const LAYERS = [
  {
    key: 'identity',
    title: 'Identity',
    fields: [
      { key: 'name', label: 'Project name', type: 'text' },
      { key: 'tagline', label: 'One-line tagline', type: 'text' },
      { key: 'category', label: 'Route', type: 'enum', options: ['marketing-site', 'saas-app'],
        help: 'Two separate routes, not one flow with a toggle — a marketing site and a SaaS app want different layers and compile differently. Picked first; everything after is route-specific.' },
      { key: 'description', label: 'One-paragraph brief (drives every suggestion below)', type: 'text' },
    ],
  },
  {
    key: 'voice',
    title: 'Voice & content',
    source: 'Balmoral competitor-copy research, 13 sites sorted into 3 registers',
    fields: [
      { key: 'copyRegister', label: 'Copy register', type: 'enum', options: ['A-institutional-authority', 'C-voice-with-a-face'],
        help: 'A states a fact and never asks for the meeting (Palantir, Bain). C is warm and specific, names the enemy directly (hybd, goodspeed). Register B, the interchangeable "Innovative Software Solutions" middle every generic AI agency site falls into, is deliberately not offered here.' },
      { key: 'readingLevel', label: 'Reading level', type: 'enum', options: ['plain', 'technical'] },
      { key: 'ctaStyle', label: 'CTA style', type: 'enum', options: ['fact-stated', 'verb-led'] },
    ],
  },
  {
    key: 'palette',
    title: 'Palette',
    source: "Jellytot brand-model layer 0 (\"cross-cutting, binds the rest\"); base packs are the real token files in tokens/",
    fields: [
      { key: 'basePack', label: 'Base style pack', type: 'enum', options: ['placeholder', 'geist', 'balmoral', 'jellytot'] },
      { key: 'groundColor', label: 'Ground / background', type: 'color' },
      { key: 'surfaceColor', label: 'Surface (cards, raised areas)', type: 'color' },
      { key: 'inkColor', label: 'Ink (body text)', type: 'color' },
      { key: 'accentColor', label: 'Accent', type: 'color' },
      { key: 'accentInkColor', label: 'Text on accent', type: 'color' },
      { key: 'borderColor', label: 'Border / rule', type: 'color' },
    ],
  },
  {
    key: 'typography',
    title: 'Typography',
    fields: [
      { key: 'displayFont', label: 'Display / body font', type: 'text' },
      { key: 'monoFont', label: 'Mono font', type: 'text' },
      { key: 'pairingStyle', label: 'Pairing style', type: 'enum', options: ['single-family', 'display-plus-body-pair'] },
      { key: 'typeScale', label: 'Type scale', type: 'enum', options: ['compact', 'comfortable', 'spacious'] },
    ],
  },
  {
    key: 'spacing',
    title: 'Spacing & shape',
    fields: [
      { key: 'spaceUnit', label: 'Base spacing unit (px)', type: 'number' },
      { key: 'density', label: 'Density', type: 'enum', options: ['compact', 'comfortable', 'spacious'] },
      { key: 'cornerRadius', label: 'Corner radius language', type: 'enum', options: ['sharp-0', 'soft-6', 'round-16', 'pill'],
        help: 'sharp-0 is Balmoral\'s own binding decision ("no rounded corners on any rectangle, panel or frame"); the others are the live open question ("rounded buttons, square?").' },
      { key: 'borderWeight', label: 'Border weight', type: 'enum', options: ['hairline', '1px', 'bold-2px'] },
      { key: 'elevation', label: 'Elevation style', type: 'enum', options: ['flat', 'soft-shadow', 'hard-offset'] },
    ],
  },
  {
    key: 'imagery',
    title: 'Imagery & iconography',
    fields: [
      { key: 'imageTreatment', label: 'Image treatment', type: 'enum', options: ['photography', 'line-art', 'duotone', 'abstract-pattern'] },
      { key: 'iconStyle', label: 'Icon style', type: 'enum', options: ['outline', 'filled', 'duotone'] },
      { key: 'artDirectionNote', label: 'Art direction note', type: 'text',
        help: 'Free text pointer to a real lane if one exists, e.g. one of Balmoral\'s 5 art-bank lanes (Proof Before Letters, Ownership In Section, Chart Datum, Foolscap Ledger, Muniment Room). Not auto-populated for new brands with no art-bank of their own.' },
    ],
  },
  {
    key: 'motion',
    title: 'Motion',
    fields: [
      { key: 'motionIntensity', label: 'Motion intensity', type: 'enum', options: ['none', 'subtle', 'expressive'] },
      { key: 'transitionStyle', label: 'Transition style', type: 'enum', options: ['fade', 'slide', 'scale'] },
    ],
  },
  {
    key: 'strategy',
    title: 'Product strategy & sitemap',
    source: 'TheProductDesignPlaybook.pdf, 12 Product Strategies + 33 Modular Plays (VDS Site Builder Figma, page "Playbook")',
    fields: [
      { key: 'productStrategies', label: 'Product strategies (pick 1-3)', type: 'multi-enum', options: [
        'Onboarding', 'Trust Building', 'Monetisation', 'Conversion Optimisation', 'Activation',
        'Experience Refinement', 'Retention', 'Intent Shaping', 'Growth & Viral', 'Habit Formation',
        'Engagement', 'Premium Positioning',
      ] },
      { key: 'modularPlays', label: 'Modular plays (auto-suggested from strategies, editable)', type: 'multi-enum', options: [
        'Commitment', 'Contact Bridge', 'Deep-link', 'Discovery', 'Effort Moat', 'Empty States',
        'Fail Safe', 'Gamified Progress', 'Intent Mirroring', 'Intentional Friction', 'Investment',
        'JTBD Copywriting', 'Limited Offer', 'Loading Feedback', 'Micro Interactions', 'Momentum Bias',
        'Pattern Alignment', 'Perceived Effort Delay', 'Permission Serve', 'Personalisation',
        'Progressive Disclosure', 'Referral', 'Sandbox Experience', 'Setup Defaults', 'Shareability',
        'Small Quirk', 'Spark Curiosity', 'Success Moments', 'System Widget', 'The Paywall',
        'Time to Value', 'Value Replay', 'Variable Reward',
      ] },
      { key: 'sitemap', label: 'Block sequence (type:variant, ordered)', type: 'block-list' },
    ],
  },
  {
    key: 'componentStyle',
    title: 'Component style (SaaS)',
    source: 'Opbox COMPONENT_INVENTORY.md-derived SaaS taxonomy; buttonShape/statusBadgeStyle map onto components already built in VDS Site Builder',
    fields: [
      { key: 'buttonShape', label: 'Button shape', type: 'enum', options: ['rounded', 'square', 'pill'] },
      { key: 'tableDensity', label: 'Table density', type: 'enum', options: ['compact', 'comfortable'] },
      { key: 'navigationPattern', label: 'Navigation pattern', type: 'enum', options: ['top-nav', 'sidebar', 'both'] },
      { key: 'statusBadgeStyle', label: 'Status badge style', type: 'enum', options: ['pill', 'dot'] },
    ],
  },
];

function fieldCount() {
  return LAYERS.reduce((n, layer) => n + layer.fields.length, 0);
}

function allFields() {
  return LAYERS.flatMap((layer) => layer.fields.map((f) => ({ ...f, layer: layer.key, layerTitle: layer.title })));
}

// Two routes, not one flow with a toggle. A marketing site compiles to real static
// HTML today (12 block types, all built and verified). A SaaS app does NOT — only 2
// of the 109 cataloged SaaS component types (Gated Action Button, StatusBadge) exist
// anywhere outside Figma, so its route is honestly scoped to an app-shell compile
// (nav + sidebar, the only SaaS-adjacent blocks with real code) plus a spec record of
// every other layer, not a claim that the whole app got built.
const ROUTES = {
  'marketing-site': LAYERS.filter((l) => l.key !== 'componentStyle').map((l) => l.key),
  'saas-app': LAYERS.filter((l) => l.key !== 'imagery').map((l) => l.key),
};

module.exports = { LAYERS, ROUTES, fieldCount, allFields };
