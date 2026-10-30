# Production-readiness execution ledger

## Baseline and accounting

- Baseline: `73a60de8db9f622ed087f85341d6762e73296a59` (`master`); working tree clean before work. No user changes to carry or overwrite.
- `TODO.md` contains 171 specification listings: 14 labelled Supported, 41 Partial, 101 Unsupported, 15 Not planned. These are inventory labels, not measured conformance. Historical snapshots, duplicate CSS2/border/transform listings, and existing Not planned entries are not independent tasks.
- The initial normalized backlog contained 110 behavior units. Audit correction separates rhythmic sizing from unrelated anchor positioning (L24), yielding **111 units**, including applicability questions and architectural constraints rather than silently dropping them. Target: **89 completed units (at least 80%)**. A constrained or investigated unit is not completed. Newly discovered independent work is appended, never substituted to reduce the denominator.
- Current completion: **1/111** (T09). Complete-inventory coverage is separately **not established**: passing existing tests and struck-through properties do not certify specification-level support.
- Acceptance for every implementation: applicable W3C semantics through the public API; meaningful observable regression coverage; native and web checks; independent review for substantial changes; integrated checks. Partial subfeatures are recorded but do not close their parent unit.

## Architecture and verification baseline

- Public entry points: `Stylesheet::parse/from_file/from_url/install`, `InlineStyle::parse`, and selector/inline-style widget wrappers in `src/lib.rs`. `from_file` is native-only; HTTP/image/font loading has native/web paths.
- `src/css` parses selectors, declarations, media, colors, images and variables. `src/layout_style` and `src/formatting.rs` resolve widget box/text styles. `src/layout_tree.rs` bridges Taffy; `src/format_layout.rs` handles specialized formatting. `src/layout` owns measurement, interaction, painting, scroll and invalidation integration. Animation/transition modules sample styles and schedule repaints.
- No crate-defined feature flags. Dependencies explicitly enable Taffy block/float/flex/grid and egui fonts. Native and wasm differ in executors, GPU/browser testing, and filesystem availability.
- `mise run check`: Clippy all targets/features/workspace with warnings denied, Cargo formatting, oxfmt. Pass policy is enforced by `clippy.toml`; no exceptions may be added. Native: `cargo test --all-features --all-targets`. Web: `mise run test:wasm` uses wasm-pack/headless Chrome. Example: `examples/app.rs`, shared theme; Trunk targets `examples/index.html`.
- Baseline native **PASS**: 14 library + 73 layout + 241 rendering + 11 example tests, 339 total, zero failures/ignored; `/tmp/opencode/ecss-baseline-native.log`.
- Baseline `mise run check` **PASS** (Clippy, rustfmt, oxfmt); `/tmp/opencode/ecss-baseline-check.log`.
- Baseline `mise run test:wasm` **PASS** in headless Chrome: 73 layout + 239 rendering + 9 example tests, 321 total, zero failures/ignored; `/tmp/opencode/ecss-baseline-web.log`. Native-only tests and ordinary library `#[test]` cases are not counted as browser execution. Linux and wasm targets installed. Other native operating systems not runtime-verified here.
- Tests use public geometry, interaction and automated GPU pixel assertions. No screenshots, image files, visual inspection or screenshot reasoning.

## Normalized remaining behavior units

All units are open unless an evidence-backed completion is recorded below. Dependencies name the prerequisite behavior, not a requirement to implement an entire newer specification first. Coordinator owns unassigned units, architecture, integration, TODO status and this ledger.

### Core and computed values (12)

| ID  | Behavior and acceptance boundary                                                                                      | Dependencies / ownership                                                  |
| --- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| C01 | Cascade priority, inheritance, CSS-wide keywords and shorthand resets across ordinary/inline/custom declarations      | Cascade agent, first batch                                                |
| C02 | Cascade layers, ordering and `revert-layer`                                                                           | C01                                                                       |
| C03 | Scoped cascade proximity and scoped rules                                                                             | Q06, C01                                                                  |
| C04 | CSS Syntax tokenization, escapes, invalid-rule/declaration recovery consistent with an explicit public parse contract | Parser contract audit; strict supported-value errors currently documented |
| C05 | Values 3/4 remaining relative units, percentage/computed-value and dimensional edge cases                             | Layout/text contexts                                                      |
| C06 | Values 5 interpolation of intrinsic sizes and new value functions                                                     | Sizing and animation                                                      |
| C07 | Custom-property substitution, invalid-at-computed-value behavior and inheritance completeness                         | C01                                                                       |
| C08 | CSS math malformed/boundary inputs and finite layout results                                                          | C05                                                                       |
| C09 | CSS functions and mixins                                                                                              | C02, C07                                                                  |
| C10 | Environment variables and fallback values                                                                             | Host environment mapping                                                  |
| C11 | Linked parameters                                                                                                     | Resource identity and URL parameter model                                 |
| C12 | Generated/replaced content, quotes and generated strings                                                              | Structured generated-content nodes                                        |

### Selectors and conditional rules (12)

| ID  | Behavior and acceptance boundary                                                              | Dependencies / ownership                                 |
| --- | --------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| Q01 | Media Queries 3/4 feature and logical-condition completeness beyond existing viewport queries | Host device capabilities                                 |
| Q02 | Media Queries 5 user preferences and device/environment features                              | Q01, host environment                                    |
| Q03 | Selectors 3/4 remaining structural, relational and attribute matching                         | Widget selector tree/metadata                            |
| Q04 | Selectors 5 additional selector semantics                                                     | Q03                                                      |
| Q05 | Namespaced selectors                                                                          | Explicit widget namespace model                          |
| Q06 | Scoping and shadow-part boundaries                                                            | Widget tree encapsulation; no DOM/shadow root API exists |
| Q07 | `@supports` declaration and logical conditions (Conditional 3)                                | Conditional agent, first batch                           |
| Q08 | Selector feature queries (Conditional 4)                                                      | Q03; conditional agent if sound                          |
| Q09 | Size/style container queries and container units (Conditional 5/Containment 3)                | Containment and layout invalidation                      |
| Q10 | CSS nesting and nested selector specificity                                                   | Q03, C01                                                 |
| Q11 | Pseudo-elements beyond partial first-line/first-letter painting                               | Generated content and text layout                        |
| Q12 | Non-element selectors and custom-highlight ranges                                             | Text range identity; browser DOM ranges unavailable      |

### Layout (24)

| ID  | Behavior and acceptance boundary                                            | Dependencies / ownership                                      |
| --- | --------------------------------------------------------------------------- | ------------------------------------------------------------- |
| L01 | Float/clear, block formatting, margin collapse and box geometry defects     | Layout agent, first batch                                     |
| L02 | Intrinsic sizing and percentages across arbitrary widget/flex/grid contexts | Layout agent audit; Taffy/measurement bridge                  |
| L03 | Grid 1 sizing/alignment completion                                          | L02                                                           |
| L04 | Grid 2 subgrid track contribution and nested interactions                   | L03                                                           |
| L05 | Grid 3 item/track packing semantics                                         | L03; evolving specification                                   |
| L06 | Flex automatic minimums, wrapping and intrinsic contributions               | L02                                                           |
| L07 | Full table sizing, formatting and collapsed-border interactions             | L02                                                           |
| L08 | Inline/line-box baseline and text-box metrics, initial letters              | Text shaping/layout                                           |
| L09 | Grid-template/chained-flow fragmentation beyond atomic widgets              | Fragment model                                                |
| L10 | Multi-column fragmentation/balancing correctness                            | Fragment model                                                |
| L11 | Multi-column 2 height and wrapping                                          | L10                                                           |
| L12 | Regions compound/anonymous content fragmentation and overset behavior       | L09, L10                                                      |
| L13 | Fragmentation rules, margin-break, orphans/widows across regions/pages      | L10, L12, L14                                                 |
| L14 | Paged layout, page breaks, page floats and page assignment                  | No page-output API; requires explicit page formatting context |
| L15 | Exclusion wrapping and CSS Shapes                                           | Float/inline fragmentation                                    |
| L16 | Alignment safety, baselines and remaining layout contexts                   | L02, L08                                                      |
| L17 | Logical properties/writing-mode interactions across all supported layouts   | L01, L02, T11                                                 |
| L18 | Containment and intrinsic fallback sizes/content visibility                 | Measurement, containing-block and paint isolation             |
| L19 | New sizing 4 size/min/max/frame semantics                                   | L02, L18                                                      |
| L20 | Overflow 4 line clamp, block ellipsis and continuation                      | Text/fragment layout                                          |
| L21 | Overflow 5 scroll markers/targets and axis lock                             | Scroll interaction model                                      |
| L22 | Display/position correctness, reading order and top-layer overlay           | Interaction/paint ordering; egui has no CSS top-layer API     |
| L23 | Anchor positioning 1/2                                                      | Cross-node geometry and dependency cycles                     |
| L24 | Rhythmic block/line-step sizing                                             | Text metrics and block formatting                             |

### Color, images and painting (16)

| ID  | Behavior and acceptance boundary                                                | Dependencies / ownership                                       |
| --- | ------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| R01 | Color 3 completeness and malformed/boundary color values                        | Existing parser/runtime color audit                            |
| R02 | Color 4 additional color spaces and conversions                                 | Coordinator; bounded sRGB output must be documented            |
| R03 | Color 5 mixing, relative colors and contrast functions                          | R02                                                            |
| R04 | Color adjustment, color schemes, forced colors and HDR limits                   | Host theme/output capability mapping                           |
| R05 | Background 4 position/repeat longhands and layer shorthand                      | Cascade/shorthand integration, logical axes                    |
| R06 | Background text and border-area clipping                                        | Glyph/border coverage masks                                    |
| R07 | Borders 4 remaining image-1D and shape/style semantics                          | Computed values and decoration paint                           |
| R08 | Image orientation/rendering/resolution and remaining Images 3/4 semantics       | Resource decoding and texture sampling                         |
| R09 | Compositing, isolation and blend modes                                          | Offscreen/layer compositing; egui paints immediate shapes      |
| R10 | Filter effects                                                                  | R09                                                            |
| R11 | Clip/clip-path and hit-test clipping                                            | Shape geometry and interaction                                 |
| R12 | Masks and mask borders                                                          | R09, R11                                                       |
| R13 | CSS fill/stroke, SVG markers/strokes                                            | Vector resource model; resvg support is not widget CSS support |
| R14 | Gap decorations for row/column/grid/flex gaps                                   | Track geometry and paint                                       |
| R15 | Cursor, pointer events, interactivity, user selection, resize and UI navigation | egui response/event lifecycle                                  |
| R16 | Caret/accent/appearance and form-control styling                                | Widget-specific host APIs; window-drag host integration        |

### Text, fonts and generated lists (19)

| ID  | Behavior and acceptance boundary                                   | Dependencies / ownership                                        |
| --- | ------------------------------------------------------------------ | --------------------------------------------------------------- |
| T01 | Font shorthand and font selection/stretch/size-adjust              | Font assets, computed text metrics                              |
| T02 | Font features, variants and synthesis                              | Shaping engine support                                          |
| T03 | Variable/optical fonts, palettes and language/emoji selection      | T02, font decoder/shaper                                        |
| T04 | Font loading descriptors, fallback and load invalidation           | Async assets and frame lifecycle                                |
| T05 | Lists, markers and counter increment/reset/set                     | Generated-content tree                                          |
| T06 | Custom counter styles                                              | T05                                                             |
| T07 | Ruby layout                                                        | Inline annotation model                                         |
| T08 | Hyphenation and language-aware line breaking                       | Language metadata/dictionaries                                  |
| T09 | Last/forced-line alignment and text-align shorthand semantics      | Text agent, first batch                                         |
| T10 | Hanging punctuation and justification completeness                 | Line breaking/shaping                                           |
| T11 | Bidi, vertical glyph orientation and text-combine-upright          | Shaping, direction and writing axes                             |
| T12 | Text emphasis marks                                                | Glyph placement, paint and vertical layout                      |
| T13 | Text shadows                                                       | Text paint and blur                                             |
| T14 | Underline position/offset and decoration skip/inset semantics      | Text decoration paint                                           |
| T15 | Text 4 whitespace collapse/trim and wrap mode/style                | Line-breaking and intrinsic measurement                         |
| T16 | Text 4 autospace, spacing trim, line padding and grouping          | Script-aware shaping/line layout                                |
| T17 | Line grid and snapping                                             | Cross-line baseline context                                     |
| T18 | Paged generated strings, running elements, footnotes and bookmarks | L14, C12                                                        |
| T19 | Hyperlink presentation                                             | Link identity/navigation model not exposed by selector metadata |

### Animation (13)

| ID  | Behavior and acceptance boundary                                      | Dependencies / ownership                                  |
| --- | --------------------------------------------------------------------- | --------------------------------------------------------- |
| A01 | 2D transforms, origins/boxes and transformed hit testing              | Paint, geometry and response mapping                      |
| A02 | 3D transforms, perspective and backface visibility                    | 3D scene/compositing                                      |
| A03 | Transition computed-value coverage and interruption correctness       | Computed property values                                  |
| A04 | Transition 2 discrete transitions and starting styles                 | A03, cascade                                              |
| A05 | Animation 1 underlying values and computed interpolation completeness | C01, computed values                                      |
| A06 | Animation 2 composition/addition and extended timing                  | A05                                                       |
| A07 | Web Animations 1 timing/control integration                           | New public animation handle model                         |
| A08 | Web Animations 2 grouped effects/timelines                            | A07                                                       |
| A09 | Motion paths                                                          | A01, path geometry                                        |
| A10 | Animation worklets                                                    | No script/worklet runtime; host-callback API needs design |
| A11 | View transitions 1/2                                                  | Cross-state retained scene/compositing                    |
| A12 | Scroll-driven animation/timelines/ranges                              | Scroll geometry, A05                                      |
| A13 | Image animation playback control                                      | Animated resource decoding/scheduling                     |

### Scrolling and viewport (7)

| ID  | Behavior and acceptance boundary                         | Dependencies / ownership                                     |
| --- | -------------------------------------------------------- | ------------------------------------------------------------ |
| S01 | Zoom and coordinate-space scaling                        | Layout/paint/input mapping                                   |
| S02 | Fullscreen integration                                   | Browser/native window host capability; no window-control API |
| S03 | Scroll snap with logical margins/padding/alignment/stops | Scroll geometry/interaction                                  |
| S04 | Initial/start scroll targets                             | S03, stable node identity                                    |
| S05 | Spatial navigation                                       | Focus graph and geometry                                     |
| S06 | Scroll anchoring                                         | Stable node identity and dynamic layout                      |
| S07 | Round-display boundary and shape-inside layout           | Nonrectangular viewport/inline geometry                      |

### APIs and integration (8)

| ID  | Behavior and acceptance boundary              | Dependencies / ownership                                               |
| --- | --------------------------------------------- | ---------------------------------------------------------------------- |
| I01 | CSSOM stylesheet/rule inspection and mutation | Existing public stylesheet is opaque; design Rust contract             |
| I02 | CSSOM View and geometry interfaces            | Persistent element identity; existing egui responses expose rectangles |
| I03 | Layout worklet API                            | Host callback lifecycle and custom layout constraints                  |
| I04 | Painting API                                  | Host callback registration and deterministic paint invalidation        |
| I05 | Registered custom properties and typed values | C07, computed value schema                                             |
| I06 | Typed OM                                      | I01, I05                                                               |
| I07 | Worklet execution                             | No JavaScript engine on native; cross-platform host model required     |
| I08 | Resize observation                            | Stable identities and post-layout notification semantics               |

## Current batch and ownership

- Conditional/background agent: `/tmp/opencode/ecss-conditional`, agent `ses_e731dcf1affet6WCgk5g3sBepa`; Q07/Q08 commit integrated, now implementing R05 on `backlog/background-axes` from `c22cd30`. Owns background axis parser/declarations/painting and tests.
- Cascade: `/tmp/opencode/ecss-cascade`, branch `backlog/cascade-correctness`, agent `ses_e731dcf05ffe2WfpoMK9UMuzgc`; owns priority/computed cascade changes and regressions. C01.
- Text/font agent: `/tmp/opencode/ecss-text`, agent `ses_e731dce9dffe7YZMGtNnRB1B7b`; T09 commit integrated, now implementing font shorthand (part of T01) on `backlog/font-shorthand` from `c22cd30`.
- Layout: `/tmp/opencode/ecss-layout`, branch `backlog/layout-correctness`, agent `ses_e731dce4bffepmgiPdkZJEX6r3`; owns layout algorithms and geometry regressions. L01/L02.
- Each worktree starts from the clean baseline. Agents commit only owned changes for explicit coordinator integration; shared parser/style points are merged serially and reviewed. No recursive delegation or push/release.
- Build isolation correction: sharing one Cargo target across these same-name packages reused stale ecss artifacts. All agent checks now use their own worktree `target`; coordinator uses `/tmp/opencode/ecss-coordinator-target`, seeded from dependency artifacts then `cargo clean -p ecss`. Post-baseline shared-target results are not accepted as verification; isolated reruns are required. The clean baseline checks finished before this issue affected implementation verification.
- Coordinator: source/support audit, baseline checks, color work, integration, independent review dispatch, TODO and ledger.

## Evidence, limitations and next actions

- Baseline check/native/web all pass as recorded above. Integrated work and remaining review findings follow.
- Integrated `ad39935` (agent `9a34ee4`): excludes inline floated boxes from the inline grouping predicate. Agent isolated evidence: 74 layout tests + targeted float rendering + wasm library compile pass; reverting the fix makes its new geometry regression fail. Independent review subsequently found the test insufficient (below). L01 remains open.
- Float review **found blocking incompleteness**: engine resets inline float dimensions, singleton grouping can rewrap floats, and nested-inline floats remain unsupported. The first regression can pass with zero size. Layout agent is correcting blockification/grouping with nonzero-size and mixed-run tests before the patch can be accepted as complete; preserve its separate intrinsic-sizing work. The earlier test pass alone is insufficient evidence.
- Integrated `c714942` (agent `30ade7c`): `@supports` declaration/logical and selector conditions, media/keyframe nesting; agent reports 77 isolated layout tests + 4 media rendering tests and native/wasm library checks. Independent reviewer `ses_e7306f3abffeiahRwDrNAK0ACa` owns acceptance of Q07/Q08.
- Integrated `c22cd30` (agent `eae9375`): text-align-last and shorthand/forced-line/justification behavior, inheritance and direction mapping. Agent reports native all-target tests and wasm library check pass. Independent reviewer `ses_e7306f359ffehhdaqm46JCDmfc` owns acceptance of T09 and checks the changed rendering fixture.
- **T09 COMPLETE**: independent review found no implementation defect; requested direct unoverridden `justify-all` coverage was added in `ba20379` and passed, along with a Clippy cleanup. Integrated native and Chrome tests verify forced/final lines, inheritance/resets, RTL/vertical mapping and justification. CSS Text remains Partial for its other units.
- Color regression evidence: initial three tests fail for unsupported modern HSL/HWB and accepted invalid mixed legacy RGB (`/tmp/opencode/ecss-color-before.log`); isolated corrected tests pass (`/tmp/opencode/ecss-color-isolated.log`). Independent reviewer `ses_e73173b7effePo6cJ2RI5DJTeW` identified premature HSL clamping and missing numeric/HWB coverage; both were corrected, and all three tests passed again. R01/R02 remain partial; additional spaces, missing components, color math and gamut handling are still outstanding.
- Math regression: `min(10px, calc(NaN * 1px))` incorrectly yields width 10 instead of 0 (`/tmp/opencode/ecss-math-before.log`). Coordinator fixes NaN propagation, top-level finite results (including retained object positions), large hypot, signed/infinite round/mod behavior, and math-function property boundaries. New nonfinite geometry test, color tests and extended object-position pixel test passed before the final property-boundary addition; final combined verification pending. Same reviewer owns math review.
- Layout agent is correcting float review findings before completing its separate intrinsic/flex/grid sizing work. Conditional agent is correcting supports review findings before continuing background axes. Text agent implements font shorthand; cascade agent finishes its first priority/cascade batch.
- Intermediate integrated checks (before supports/text integration and final math-review fixes): native library 14 + layout 76 + rendering 244 **PASS**; example 9 pass / 2 fail. Animation CPU p95 **21.407329 ms** exceeded the unchanged 16.666667 ms gate; the scroll test then failed from its poisoned performance lock. Concurrent agent compilation/browser checks were active, but causation is not established. Rerun performance in a quiet interval and compare baseline if needed; do not suppress/relax assertions. `/tmp/opencode/ecss-batch1-native.log`.
- Same intermediate tree browser runtime **PASS**: 76 layout + 242 rendering + 9 example tests (327), `/tmp/opencode/ecss-batch1-web.log`. This is not final-tree verification.
- Math reviewer found literal negative-zero and retained nonfinite-value gaps. Literal `-0` width regression reproduced (expected width20, actual0) in `/tmp/opencode/ecss-math-review-before.log`. Corrections canonicalize literal zero, preserve computed signed zero internally, censor retained constant math before interpolation and route nonfinite retained percentages through the resolver. Added numeric flex-factor coverage and a bounded-complexity parser/error regression. Final math checks/re-review pending.
- Six math behavioral tests now pass in native and Chrome (the sixth covers large retained percentages). Math changes are committed as `cc362ce`. C08 broader semantics remain open; do not infer full math conformance from these regressions.
- Color changes committed as `a647fb6`; independent review confirms both color findings resolved. Whole R01/R02 units remain partial.
- Integrated `mise run check` **PASS**, `/tmp/opencode/ecss-integrated-check.log`. This includes supports/text/color and math corrections; the last percentage fast-path guard and float/supports followups require their own integrated checks.
- Latest full native check **PASS**: 14 library + 89 layout + 244 rendering + 11 example tests (358), `/tmp/opencode/ecss-integrated-native.log`. Both unchanged performance gates pass: save animation p95 **14.600911 ms**, scroll p95 **12.582177 ms**. The earlier failure remains recorded; this passing rerun does not establish universal performance on other machines.
- Integrated Chrome runtime **PASS**: 88 layout + 242 rendering + 9 example tests (339), `/tmp/opencode/ecss-integrated-web.log`. The subsequent focused layout target passes all 89 tests, including the last large-percentage regression, `/tmp/opencode/ecss-math-browser.log`. A whole-suite checkpoint run is in `/tmp/opencode/ecss-checkpoint-web.log`.
- Math review confirms earlier findings resolved; it also identified direct retained large percentages bypassing the clamp. Ratios above 100% now retain resolver-based clamping after the basis is known; new transition geometry coverage for `1e15%` and `1e38%` passes in the full native run. Broader C08 semantics remain open rather than claiming complete CSS Math support.
- Supports review found five issues: general-enclosed conditions error instead of false, overrestrictive unknown-function filtering with valid var(), reserved `--` accepted, unknown group at-rules abort parsing, multiline error locations point to line1. Original conditional agent is correcting these before continuing background axes; Q07/Q08 are not complete.
- Context7 resolution for cssparser failed due to the service monthly quota. Fallback documentation: versioned `https://docs.rs/cssparser/0.38.0/cssparser/enum.Token.html`; W3C CSS Color 4 sections 4.1, 7 and 8 were consulted directly. Context7 unavailability is not represented as a successful documentation lookup.
- Strong existing automated coverage does not imply production readiness. Complete specification support claims require correction where known gaps exist; in particular Color 3/4 and Media Queries labels understate existing partial behavior, while whole-module Supported claims need scrutiny.
- Browser DOM, shadow roots, worklets, retained element ranges, page output and window control are not current ecss abstractions. Their units remain visible and counted pending an evidence-backed applicability decision; no native/web support is inferred from browser-only APIs or resource-decoder capabilities.
- Next: finish focused large-percentage browser validation and commit math; integrate/review cascade, corrected floats, corrected supports, font shorthand and background axes as agents hand off; run combined checks and update precise support claims; continue dependency-ready implementation batches.
- Production-readiness gate: **not met**. Backlog target, full support-claim audit, performance assessment and final integrated platform checks remain outstanding.

## Resumable checkpoint — agent usage limit

All four implementation agents stopped with **“The usage limit has been reached”**. No implementation or review agents are active. Their interrupted changes were committed only on their isolated branches to preserve them; these WIP commits are **unverified and not integrated**:

| Work                 | Directory / branch                                           | WIP commit | Contents and next action                                                                                                                                                                                                                              |
| -------------------- | ------------------------------------------------------------ | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Cascade              | `/tmp/opencode/ecss-cascade` / `backlog/cascade-correctness` | `cdd691e`  | New `src/css/cascade.rs`, declaration priority representation, layout/visual/pseudo resolution and property regressions. Finish isolated validation and independent review; integrate before font/background declaration changes. Base is `73a60de8`. |
| Floats/sizing        | `/tmp/opencode/ecss-layout` / `backlog/layout-correctness`   | `19fc41f`  | Inline float engine/grouping corrections and expanded geometry tests, plus exploratory grid/sizing tests. Finish mixed/nested float behavior; confirm tests assert actual nonzero sizes and wrapping. Base includes `9a34ee4`, not text-align-last.   |
| Supports/backgrounds | `/tmp/opencode/ecss-conditional` / `backlog/background-axes` | `29bc1cb`  | Five supports review corrections are interleaved with substantial unfinished axis/background parser and painting work. Separate, validate and review supports corrections first, then complete R05 with pixel coverage. Base is `c22cd30`.            |
| Font shorthand       | `/tmp/opencode/ecss-text` / `backlog/font-shorthand`         | `2c346bf`  | New `src/css/font.rs`, shorthand/computed style/font loading changes and layout/rendering regressions. Finish and review after cascade integration. Base is `c22cd30`.                                                                                |

- Main implementation commits after baseline: `ad39935`, `c714942`, `c22cd30`, `a647fb6`, `ba20379`, `cc362ce`. The first float/supports commits still have the concrete outstanding review findings recorded above. Only T09 is counted fully completed.
- All baseline user files were initially clean. No unrelated user changes were reverted. Worktree WIP source stays off `master`; do not blindly cherry-pick those checkpoints as finished work.
- Resume agent conversations by the IDs in ownership above after capacity is available; tell them their interrupted work is now committed at the WIP hashes. Review conversation IDs are recorded in evidence above. Coordinator can also finish work directly, but substantial changes still require independent review.
- Use each worktree's own `target` and coordinator `CARGO_TARGET_DIR=/tmp/opencode/ecss-coordinator-target`. Never share ecss build artifacts between source worktrees. `wasm-pack test` forwards positional filters to `cargo build`, so a test-name filter fails; `-- --test layout` runs the full layout target successfully.
- Resume order: cascade corrections; float/supports review blockers; font/background completion; combined lint/native/Chrome checks; TODO/ledger reconciliation; next dependency-ready units. Existing Project knowledge's border-conformance todo remains separate and open.
- Final checkpoint source checks: native **358 passed**, `mise run check` **passed** (`/tmp/opencode/ecss-checkpoint-check.log`), whole-suite Chrome **340 passed** (89 layout + 242 rendering + 9 example; `/tmp/opencode/ecss-checkpoint-web.log`). Zero failed/ignored tests in these final runs. Windows/macOS runtime verification remains unavailable in this Linux environment.
