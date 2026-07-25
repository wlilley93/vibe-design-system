"""The VDS proof scripts.

The registry is CLOSED (VDS S-7(5)). Adding a kind amends the specification and
the invariant registry; it is not a script anyone may drop in, because an open
registry is a free-form script surface and a free-form script surface is not a
gate (VDS S-7(6)).

Implemented here: register_completeness, composition, states.
Specified and NOT implemented here: reconciliation, contrast, parity, token_pin,
retirement_drain, ledger_staleness, no_stored_values.
"""

__all__ = ["composition", "register_completeness", "states"]
