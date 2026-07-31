# examples/opbox

Opbox's design kit, imported into a register. **Read-only into this repository:
nothing here is written back to Opbox.**

`src/components/ui/` holds five `.tsx` files copied verbatim out of
`~/Projects/Opbox/design-deliverables/2026-07-24/opbox-deliverables.zip`. They
are here as EVIDENCE, so `vds register import` can be run against a third
codebase that this programme did not author, and so the run reproduces.

## What it is for

site-factory is my own code and the Simple Design System harvest is a library
plus a published repo. Opbox is the third case and the only one that is a real
in-house application kit: 21 exported components in five files, written by
somebody building a product rather than a design system.

## What the import found

    21 exported components, 5 files, 0 skipped
    55 props across 21 records, 0 records with none
    5 records flagged INCOMPLETE (their declaration extends or intersects a DOM type)

It also found the scanner's third gap. Two components declare a named
`ButtonProps` / `InputProps`; the other **nineteen annotate the destructured
parameter with an anonymous type literal**, which the named-type reader could
not see. Before the fix the import wrote 19 records carrying `props: []` - a
contract this reader could not see, recorded identically to a component that
genuinely takes nothing. `inline_prop_types_in` closed it: 7 props became 55.

## What it is not

The register is at `proposed`, every coordinate is DERIVED (the kit ships no
`app/**/page.tsx`, so no screen imports it and no coordinate is observed), and
`figma` is null on all 21. Nobody has decided their required states, contrast
floors, roles or keyboard contracts. `vds doctor` reports 3 of 10, which is
what a bare register with no vendored designpack is worth.

The design RULES the kit ships with, and the two its own CSS breaks, are
measured separately in `site-factory/vendor/opbox-rules-audit.json`.
