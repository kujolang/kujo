# Remaining collection costs — factor isolation

Measured runtime: `6d288612375548ae9ca941776d20d37df51c0420`.
[Full evidence](collection-factor-isolation-2026-10-28.json) contains 192 allocation
observations, 288 VM/interpreter/JIT semantic checks, exact fixtures, runner sources,
identities and independent matrix verification. These diagnostic variants preserve
the asserted final values; they are not substitutions in the LangArena port.

Each comparison uses dictionary widths 128/2,048 and 16/32 calls. Added-call slopes
exclude one-time setup. Initial-only and fresh-per-call snapshots are separate.
VM and JIT agree on all requested-byte/request slopes in the matrix.

## Confirmed remaining costs

Initial-only snapshots, requested bytes per added call:

| Operation | Counter source | Width 128 | Width 2,048 |
| --- | --- | ---: | ---: |
| Insert one new key | Captured state | 47,320 | 634,840 |
| Insert one new key | Primitive argument | 47,626 | 635,146 |
| Replace one value with an array | Captured state | 44,546 | 632,066 |
| Replace one value with an array | Primitive argument | 44,824 | 632,344 |
| Eight compound indexed updates | Captured state | 45,491 | 633,011 |
| Eight compound indexed updates | Primitive argument | 45,769 | 633,289 |
| Eight direct indexed assignments | Captured state | 45,491 | 633,011 |
| Eight direct indexed assignments | Primitive argument | 7,307 | 7,307 |

The affected rows have a 587,520-byte width penalty per added call. Dynamic loop
bounds alone are not the problem: direct assignments using only primitive
parameters already have flat width costs. Removing either the captured counter or
the compound indexed expression alone does not remove the loop penalty here.
The counter control replaces both its captured update and return read with a
primitive parameter; it does not distinguish those two effects by itself.

Source review explains the relevant admission boundaries:

- Initial dictionary publication requires an existing key and a supported scalar
  value. Insertion and initial array replacement fall back.
- The batching profitability proof rejects indexed values used inside arithmetic.
- Whole captured-value reads in the proof must be parameters. A captured counter
  therefore also prevents the optimized multi-key route in these fixtures.

Do not simply remove existing-key admission: preparation, promotion, snapshots and
publication currently assume existing keys. Any insertion change must update those
paths or retain safe fallback. Any broader batching proof must preserve callback,
observation, error and snapshot behavior and avoid new fallback allocation costs.

## Dense work and required snapshots

Dense loops perform 128 versus 2,048 writes, so their bytes per call should not be
flat. The direct-assignment/primitive-argument control uses 23,331/278,755 bytes
and 201/2,125 requests respectively. These two points must be interpreted against
the changed write count; they are not proof of universal linear scaling.
Fresh snapshots retain separate old values and remain explicit semantic controls.
No elapsed-speed, clean-host or production benchmark claim follows.

## Path control and next baseline

The first refresh used a longer output directory, adding a constant 480 bytes to
raw setup totals while leaving slopes unchanged. Comparing both frozen runtimes
on identical fixture paths removes that difference: all 192 paired observations
match exactly between `4d11952` and `6d28861`. Source hashes alone would not have
controlled allocations caused by source-path strings. Future before/after probes
must use identical paths as well as identical source and build identities.

The remaining priorities are new-key insertion, nonscalar replacement, and safe
batching with captured scalar state or indexed arithmetic. The shared capture
metadata fix is already accepted; it does not close these collection costs.
Sections 3 and 4 remain open pending fixes, regression gates and representation
review. Use the frozen `6d28861` artifacts as the next before-fix baseline.
