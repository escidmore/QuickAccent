# Special characters

Enable the sets you want in `~/.config/quickaccent/config.toml`:

```toml
languages = ["French", "Special", "Currency", "Typography", "Arrows", "Math", "CurrencyExtended"]
```

The language list reloads automatically. Sets merge in configuration order,
with duplicate choices removed. Put your most-used set first. Existing configs
keep their current sets.

Hold a base key for your configured hold delay, press your activation key
(Space by default), cycle with Space or arrows, then release the base key.
Punctuation and number bindings use physical US key positions. On macOS, letters
also use physical US positions. Non-US layouts may have different legends.
Shift changes case, not the punctuation binding; use `=` without Shift for Plus.

## PowerToys sets

`Special` and `Currency` use the corresponding
[PowerToys tables](https://github.com/microsoft/PowerToys/blob/main/src/modules/poweraccent/PowerAccent.Common/CharacterMappings.cs).
Other PowerToys language tables are not imported by this change.

- `.`: ellipses and combining accents.
- `-`: en/em/two-em/three-em dashes, nonbreaking hyphen, mathematical minus.
- `=`: comparisons and operators, including ≤ ≥ ≠ ≈ ± ≡.
- `,`: primes, angle quotation marks and mathematical symbols.
- `/`: ÷ √ ‽ ⸘; keypad divide: ÷ √; keypad multiply: × ⋅ ˣ ₓ.
- Backslash: grave accent and tilde.
- Number row: subscripts, superscripts and fractions; `0` includes °, `8` includes ∞.
- Letters: examples include `c` → © °C, `f` → °F, `r` → ®, `t` → ™, `s` → § ∑ ∫.
- Currency examples: `e` → €, `p` → £ ₽ ₱, `r` → ₹ ៛ ﷼, `s` → $ ₪, `y` → ¥.

Some currency choices are abbreviation letters, matching PowerToys. Standalone
combining accents appear on a dotted circle in the picker; only the accent is
inserted, attaching to the preceding character. Display direction marks are
never inserted. Shift preserves complete multi-character choices such as °C.

## Optional extensions

Each extension can be enabled independently. These are all its bindings, in
picker order. The quote key is the US apostrophe/double-quote key, without Shift.

| Set | Base key | Choices |
| --- | --- | --- |
| Typography | `'` | ‘ ’ “ ” „ ‚ « » ‹ › |
| Typography | `.` | • ◦ ▪ |
| Typography | `t` | † ‡ ※ |
| Typography | `v` | ✓ ✔ |
| Typography | `x` | ✗ ✘ |
| Arrows | `-` | ← → ↑ ↓ ↔ ↕ ↗ ↘ ↙ ↖ |
| Arrows | `=` | ⇒ ⇐ ⇔ |
| Math | `=` | ≔ ≝ ≟ ≢ ∝ |
| Math | `,` (the `<` key) | ≪ ≲ ⊂ ⊆ |
| Math | `.` (the `>` key) | ≫ ≳ ⊃ ⊇ |
| Math | `i`, `u` | ∩ / ∪ |
| Math | `a`, `o`, `n` | ∧ / ∨ / ¬ |
| Math | `t` | ∴ ∵ |
| CurrencyExtended | `a` | ؋ (afghani) |
| CurrencyExtended | `b` | ₿ (bitcoin) |
| CurrencyExtended | `c` | ₵ (cedi), ¤ (generic currency) |
| CurrencyExtended | `d` | ֏ (dram) |
| CurrencyExtended | `g` | ₲ (guaraní) |
| CurrencyExtended | `l` | ₾ (lari) |
| CurrencyExtended | `n` | ₦ (naira) |
| CurrencyExtended | `r` | ₨ (rupee) |
