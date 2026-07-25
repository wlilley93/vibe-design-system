# storefront: VDS applied to a project that has a surface

This is not a toy and it is not documentation. It is the only place in this
repository where every implemented proof kind runs over rows that exist, and CI
runs it with **no `--allow-vacuous`**, so a vacuity here is a red build.

## Why it exists

VDS is a governance kernel. It has no screens and no component library, so five
of the seven implemented kinds are legitimately vacuous when run against VDS
itself, and `.vds/config.toml` at the repository root says so at length. A
vacuous pass is not evidence (S-7(2)(4)). On its own, therefore, the repository
could not demonstrate that its own gates catch anything: `vds doctor` reported
D1, D2 and D3 unmet, and the honest reason was "there is nothing here to
govern".

This subject is the answer. Three screens, six components, two themes, and one
deprecated record draining to zero, chosen so that each kind has something real
to be right or wrong about:

| kind | what it enforces here | rows |
|---|---|---|
| `register_completeness` | every component the three screens reference is registered | 15 |
| `reconciliation` | the register and `src/components/ui` agree, both directions | 12 |
| `composition` | no screen reaches for an unregistered component | 15 |
| `states` | every required state of every registered component is drawn | 6 |
| `retirement_drain` | `Notice`, deprecated in favour of `Alert`, has zero consumers | 1 |
| `ledger_staleness` | the screens ledger is current with the three screens | 1 |
| `no_stored_values` | this project's own `.vds/` holds no realisation | 23 |

## Where the design values live, and why not here

`app/globals.css` is full of colours, radii and durations, and that is correct.
It is the named record S-2(3) makes the system of record for what a token
resolves to. VDS reads it and does not own it.

`.vds/register/*.yaml` holds none of them. A record names
`control-boundary against surface, min_ratio 3.0, WCAG 2.2 SC 1.4.11`. That is a
REQUIREMENT: a duty imposed from outside the design, lawful under S-2(4), and a
numeral that is not a value under S-2(6). What `--sf-control-border` actually
resolves to is in the stylesheet and nowhere else, and `no_stored_values` runs
over this project's `.vds/` on every CI run to keep it that way.

Two themes exist so that `contrast`, once implemented, has something to be wrong
about: a boundary that clears its floor in light and fails in dark has failed.

## How it was built, which is the adoption path

Nothing here was hand-written into `.vds/`. Every record came out of the tool,
in the order an adopter would run it:

```sh
vds init --root examples/storefront --jurisdiction storefront --repo-code SF
vds ledger screens --root examples/storefront
vds register import --root examples/storefront
# then, per component, the contract - which is a DECISION and is the part the
# tool deliberately refuses to invent:
vds register amend --root examples/storefront CMP-0003 --kind non_breaking \
  --what "..." --add-required default,hover,focus,disabled \
  --add-drawn default,hover,focus,disabled \
  --role button --name-source children \
  --set-floor "control-boundary:surface:3.0:WCAG 2.2 SC 1.4.11 non-text contrast:control_boundary"
vds register set-status --root examples/storefront CMP-0003 designed
vds register set-status --root examples/storefront CMP-0003 registered
vds register measure-demand --root examples/storefront --all
vds proof --all --root examples/storefront
```

`register import` read all six components out of `src/components/ui` and found
five of the six import paths OBSERVED in a screen that really imports them. The
sixth, `Notice`, is imported by nothing, so its coordinate could only be DERIVED
from a naming rule and the tool deferred it until asked for it explicitly with
`--include-derived`. That distinction is the point: a guessed coordinate is
labelled as one.

## What building it found

Two defects, both of which every unit test in the repository had missed, because
every unit test started from a record that was already `registered`:

1. **A fresh candidate could never be given its contract.** `register import`
   mints at `proposed` and prints advice telling the author to set the role with
   a `non_breaking` amendment. Setting a role from null was classed breaking, a
   breaking amendment demands a warrant, and no warrant can exist over a record
   nobody has registered. Fixed by making `breaking_reasons` respect what
   S-9(4) actually says: an amendment breaks the contract `before` **published**,
   and a record below `registered` has published nothing.

2. **`NameSource` had no variant for the HTML `<label>` association**, which is
   the standard way a text input gets its accessible name. The nearest available
   value, `aria_labelledby`, is a different mechanism, so recording it would have
   published a contract the code does not keep.

That is the argument for this directory in one paragraph. Adoption finds what
unit tests cannot, and a kernel with no subject is a kernel that never adopts
anything.
