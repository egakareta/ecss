# Feature Support TODO

> **Sources:** [W3C CSS specifications index](https://www.w3.org/Style/CSS/specs.en.html) and the [W3C generated CSS property index](https://www.w3.org/Style/CSS/all-properties.en.html) ([JSON data](https://www.w3.org/Style/CSS/all-properties.en.json)). Property names are grouped by the specification title used in that index and sorted alphabetically. Repeated property names are retained under each spec that defines them. The generated index says it may omit properties, especially from Editors’ Drafts; entries can include proposed and vendor-prefixed properties. A feature without a CSS-properties sub-bullet has no separate property list in that generated index (for example, selector/API/syntax specs, profiles, and snapshots).

## Core CSS, syntax, and cascading

- **[CSS 2.1](https://www.w3.org/TR/CSS2/) - Unsupported.**
  - CSS properties: _~~`azimuth`~~_, `background`, `background-attachment`, ~~`background-color`~~, `background-image`, `background-position`,
    `background-repeat`, `border`, `border-bottom`, `border-bottom-color`, `border-bottom-style`, `border-bottom-width`,
    `border-collapse`, `border-color`, `border-left`, `border-left-color`, `border-left-style`, `border-left-width`,
    `border-right`, `border-right-color`, `border-right-style`, `border-right-width`, `border-spacing`, `border-style`,
    `border-top`, `border-top-color`, `border-top-style`, `border-top-width`, `border-width`, `bottom`, `caption-side`,
    `clear`, `clip`, ~~`color`~~, `content`, `counter-increment`, `counter-reset`, _~~`cue`~~_, _~~`cue-after`~~_, _~~`cue-before`~~_,
    `cursor`, `direction`, `display`, _~~`elevation`~~_, `empty-cells`, `float`, `font`, ~~`font-family`~~, `font-size`,
    ~~`font-style`~~, `font-variant`, ~~`font-weight`~~, ~~`height`~~, `left`, `letter-spacing`, `line-height`, `list-style`,
    `list-style-image`, `list-style-position`, `list-style-type`, ~~`margin`~~, ~~`margin-bottom`~~, ~~`margin-left`~~,
    ~~`margin-right`~~, ~~`margin-top`~~, `max-height`, `max-width`, `min-height`, `min-width`, `orphans`, `outline`,
    `outline-color`, `outline-style`, `outline-width`, `overflow`, ~~`padding`~~, ~~`padding-bottom`~~, ~~`padding-left`~~,
    ~~`padding-right`~~, ~~`padding-top`~~, `page-break-after`, `page-break-before`, `page-break-inside`, _~~`pause`~~_,
    _~~`pause-after`~~_, _~~`pause-before`~~_, _~~`pitch`~~_, _~~`pitch-range`~~_, _~~`play-during`~~_, `position`, `quotes`, _~~`richness`~~_, `right`,
    _~~`speak`~~_, _~~`speak-header`~~_, _~~`speak-numeral`~~_, _~~`speak-punctuation`~~_, _~~`speech-rate`~~_, _~~`stress`~~_, `table-layout`,
    `text-align`, `text-decoration`, `text-indent`, `text-transform`, `top`, `unicode-bidi`, `vertical-align`,
    `visibility`, _~~`voice-family`~~_, _~~`volume`~~_, `white-space`, `widows`, ~~`width`~~, `word-spacing`, `z-index`. **Not planned:** aural/speech properties.
- **[CSS Cascading and Inheritance Level 3](https://www.w3.org/TR/css3-cascade/) - Unsupported.**
  - CSS properties: `all`.
- **[CSS Cascading and Inheritance Level 4](https://www.w3.org/TR/css-cascade-4/) - Unsupported.**
  - CSS properties: `all`.
- **[CSS Cascading and Inheritance Level 5](https://www.w3.org/TR/css-cascade-5/) - Unsupported.**
  - CSS properties: `all`.
- **[CSS Cascading and Inheritance Level 6](https://www.w3.org/TR/css-cascade-6/) - Unsupported.**
- **[CSS Generated and Replaced Content](https://www.w3.org/TR/css3-content/) - Unsupported.**
  - CSS properties: `bookmark-label`, `bookmark-level`, `bookmark-state`, `content`, `quotes`, `string-set`.
- **CSS Math - Unsupported.**
- **[CSS style Attribute Syntax](https://www.w3.org/TR/css-style-attr/) - Unsupported.**
- **[CSS Syntax](https://www.w3.org/TR/css3-syntax/) - Unsupported.**
- **[CSS Values and Units Level 3](https://www.w3.org/TR/css3-values/) - Unsupported.**
- **[CSS Values and Units Level 4](https://www.w3.org/TR/css-values-4/) - Unsupported.**
- **[CSS Values and Units Level 5](https://www.w3.org/TR/css-values-5/) - Unsupported.**
  - CSS properties: `interpolate-size`.
- **[CSS Counter Styles Level 3](https://www.w3.org/TR/css-counter-styles-3/) - Unsupported.**
- **[CSS Custom Properties for Cascading Variables Level 1](https://www.w3.org/TR/css-variables/) - Unsupported.**
  - CSS properties: `--*`.
- **[CSS Will Change Module Level 1](https://www.w3.org/TR/css-will-change-1/) - Unsupported.**
  - CSS properties: `will-change`.
- **[CSS Pseudo-Elements Module Level 4](https://www.w3.org/TR/css-pseudo-4/) - Unsupported.**
- **[CSS Logical Properties and Values Level 1](https://www.w3.org/TR/css-logical-1/) - Unsupported.**
  - CSS properties: `block-size`, `border-block`, `border-block-color`, `border-block-end`, `border-block-end-color`,
    `border-block-end-style`, `border-block-end-width`, `border-block-start`, `border-block-start-color`,
    `border-block-start-style`, `border-block-start-width`, `border-block-style`, `border-block-width`,
    `border-end-end-radius`, `border-end-start-radius`, `border-inline`, `border-inline-color`, `border-inline-end`,
    `border-inline-end-color`, `border-inline-end-style`, `border-inline-end-width`, `border-inline-start`,
    `border-inline-start-color`, `border-inline-start-style`, `border-inline-start-width`, `border-inline-style`,
    `border-inline-width`, `border-start-end-radius`, `border-start-start-radius`, `inline-size`, `inset`,
    `inset-block`, `inset-block-end`, `inset-block-start`, `inset-inline`, `inset-inline-end`, `inset-inline-start`,
    `margin-block`, `margin-block-end`, `margin-block-start`, `margin-inline`, `margin-inline-end`,
    `margin-inline-start`, `max-block-size`, `max-inline-size`, `min-block-size`, `min-inline-size`, `padding-block`,
    `padding-block-end`, `padding-block-start`, `padding-inline`, `padding-inline-end`, `padding-inline-start`.
- **[CSS Functions and Mixins](https://www.w3.org/TR/css-mixins-1/) - Unsupported.**
- **[CSS Environment Variables Level 1](https://www.w3.org/TR/css-env-1/) - Unsupported.**
- **[CSS Linked Parameters Level 1](https://www.w3.org/TR/css-link-params-1/) - Unsupported.**
  - CSS properties: `link-parameters`.

## Selectors, queries, and scoping

- **[Media Queries Level 3](https://www.w3.org/TR/mediaqueries-3/) - Unsupported.**
- **[Media Queries Level 4](https://www.w3.org/TR/mediaqueries-4/) - Unsupported.**
- **[Selectors Level 3](https://www.w3.org/TR/css3-selectors/) - Unsupported.**
- **[Selectors Level 4](https://www.w3.org/TR/selectors4/) - Unsupported.**
- **[Selectors Level 5](https://www.w3.org/TR/selectors-5/) - Unsupported.**
- **[CSS Namespaces](https://www.w3.org/TR/css3-namespace/) - Unsupported.**
- **[CSS Scoping](https://www.w3.org/TR/css-scoping-1/) - Unsupported.**
- **[CSS Conditional Rules](https://www.w3.org/TR/css3-conditional/) - Unsupported.**
- **[Non-element Selectors Module Level 1](https://www.w3.org/TR/selectors-nonelement-1/) - Unsupported.**
- **[CSS Shadow Parts](https://www.w3.org/TR/css-shadow-parts-1/) - Unsupported.**
- **[CSS Conditional Rules Level 4](https://www.w3.org/TR/css-conditional-4/) - Unsupported.**
- **[CSS Conditional Rules Level 5](https://www.w3.org/TR/css-conditional-5/) - Unsupported.**
  - CSS properties: `container`, `container-name`, `container-type`.
- **[Media Queries Level 5](https://www.w3.org/TR/mediaqueries-5/) - Unsupported.**
- **[CSS Custom Highlight API Level 1](https://www.w3.org/TR/css-highlight-api-1/) - Unsupported.**
- **[CSS Nesting](https://www.w3.org/TR/css-nesting-1/) - Unsupported.**

## Box model and layout

- **[CSS Grid Template Layout](https://www.w3.org/TR/css3-layout/) - Unsupported.**
  - CSS properties: `chains`, `flow`, `grid`, `grid-template`, `grid-template-areas`, `grid-template-columns`, `grid-template-rows`.
- **[CSS Basic Box Model](https://www.w3.org/TR/css3-box/) - Unsupported.**
  - CSS properties: ~~`margin`~~, ~~`margin-bottom`~~, ~~`margin-left`~~, ~~`margin-right`~~, ~~`margin-top`~~, ~~`padding`~~, ~~`padding-bottom`~~, ~~`padding-left`~~,
    ~~`padding-right`~~, ~~`padding-top`~~.
- **[CSS Basic Box Model Level 4](https://www.w3.org/TR/css3-box/) - Unsupported.**
  - CSS properties: ~~`margin`~~, ~~`margin-bottom`~~, ~~`margin-left`~~, ~~`margin-right`~~, ~~`margin-top`~~, `margin-trim`, ~~`padding`~~, ~~`padding-bottom`~~,
    ~~`padding-left`~~, ~~`padding-right`~~, ~~`padding-top`~~.
- **[CSS Page Floats](https://www.w3.org/TR/css-page-floats-3/) - Unsupported.**
  - CSS properties: `clear`, `float`, `float-defer`, `float-offset`, `float-reference`.
- **[CSS Line Layout](https://www.w3.org/TR/css3-linebox/) - Unsupported.**
- **[CSS Multi-column Layout](https://www.w3.org/TR/css3-multicol/) - Unsupported.**
  - CSS properties: `column-count`, `column-fill`, `column-rule`, `column-rule-color`, `column-rule-style`, `column-rule-width`,
    `column-span`, `column-width`, `columns`.
- **[CSS Multi-column Layout Level 2](https://www.w3.org/TR/css-multicol-2/) - Unsupported.**
  - CSS properties: `column-count`, `column-fill`, `column-height`, `column-rule`, `column-rule-color`, `column-rule-style`,
    `column-rule-width`, `column-span`, `column-width`, `column-wrap`, `columns`.
- **[CSS Paged Media](https://www.w3.org/TR/css3-page/) - Unsupported.**
  - CSS properties: `page`.
- **[CSS Positioned Layout Level 3](https://www.w3.org/TR/css3-positioning/) - Unsupported.**
  - CSS properties: `bottom`, `inset`, `inset-block`, `inset-block-end`, `inset-block-start`, `inset-inline`, `inset-inline-end`,
    `inset-inline-start`, `left`, `position`, `right`, `top`.
- **[CSS Positioned Layout Level 4](https://www.w3.org/TR/css-position-4/) - Unsupported.**
  - CSS properties: `overlay`.
- **[Grid Layout](https://www.w3.org/TR/css-grid-1/) - Unsupported.**
  - CSS properties: `grid`, `grid-area`, `grid-auto-columns`, `grid-auto-flow`, `grid-auto-rows`, `grid-column`, `grid-column-end`,
    `grid-column-start`, `grid-row`, `grid-row-end`, `grid-row-start`, `grid-template`, `grid-template-areas`,
    `grid-template-columns`, `grid-template-rows`.
- **[Grid Layout Level 2](https://www.w3.org/TR/css-grid-2/) - Unsupported.**
  - CSS properties: `grid`, `grid-area`, `grid-auto-columns`, `grid-auto-flow`, `grid-auto-rows`, `grid-column`, `grid-column-end`,
    `grid-column-start`, `grid-row`, `grid-row-end`, `grid-row-start`, `grid-template`, `grid-template-areas`,
    `grid-template-columns`, `grid-template-rows`.
- **[Grid Layout Level 3](https://www.w3.org/TR/css-grid-3/) - Unsupported.**
  - CSS properties: `fit-tolerance`, `flow-tolerance`, `item-cross`, `item-direction`, `item-flow`, `item-pack`, `item-track`,
    `item-wrap`.
- **[Regions](https://www.w3.org/TR/css3-regions/) - Unsupported.**
  - CSS properties: `break-after`, `break-before`, `break-inside`, `flow-from`, `flow-into`, `region-fragment`.
- **CSS Tables Module - Unsupported.**
  - CSS properties: `border-collapse`, `border-spacing`, `caption-side`, `empty-cells`, `table-layout`.
- **[CSS Flexible Box Layout Level 1](https://www.w3.org/TR/css3-flexbox/) - Unsupported.**
  - CSS properties: `align-content`, `align-items`, `align-self`, `flex`, `flex-basis`, `flex-direction`, `flex-flow`, `flex-grow`,
    `flex-shrink`, `flex-wrap`, `justify-content`.
- **[CSS Fragmentation Module Level 3](https://www.w3.org/TR/css3-break/) - Unsupported.**
  - CSS properties: `box-decoration-break`, `break-after`, `break-before`, `break-inside`, `orphans`, `widows`.
- **[CSS Fragmentation Module Level 4](https://www.w3.org/TR/css-break-4/) - Unsupported.**
  - CSS properties: `box-decoration-break`, `break-after`, `break-before`, `break-inside`, `margin-break`, `orphans`, `widows`.
- **[CSS Exclusions](https://www.w3.org/TR/css3-exclusions/) - Unsupported.**
  - CSS properties: `wrap-flow`, `wrap-through`.
- **[CSS Shapes](https://www.w3.org/TR/css-shapes-1/) - Unsupported.**
  - CSS properties: `shape-image-threshold`, `shape-margin`, `shape-outside`.
- **[CSS Box Alignment Module Level 3](https://www.w3.org/TR/css3-align/) - Unsupported.**
  - CSS properties: `align-content`, `align-items`, `align-self`, `justify-content`, `justify-items`, `justify-self`, `place-content`,
    `place-items`, `place-self`.
- **[CSS Intrinsic & Extrinsic Sizing Module Level 3](https://www.w3.org/TR/css3-sizing/) - Unsupported.**
  - CSS properties: `block-size`, `box-sizing`, ~~`height`~~, `inline-size`, `max-block-size`, `max-height`, `max-inline-size`, `max-width`,
    `min-block-size`, `min-height`, `min-inline-size`, `min-width`, ~~`width`~~.
- **[CSS Box Sizing Module Level 4](https://www.w3.org/TR/css-sizing-4/) - Unsupported.**
  - CSS properties: `aspect-ratio`, `contain-intrinsic-block-size`, `contain-intrinsic-height`, `contain-intrinsic-inline-size`,
    `contain-intrinsic-size`, `contain-intrinsic-width`, `frame-sizing`, `max-size`, `min-intrinsic-sizing`, `min-size`,
    `size`.
- **[CSS Overflow Module Level 3](https://www.w3.org/TR/css-overflow-3/) - Unsupported.**
  - CSS properties: `overflow`, `overflow-block`, `overflow-clip-margin`, `overflow-inline`, `overflow-x`, `overflow-y`,
    `scroll-behavior`, `scrollbar-gutter`, `text-overflow`.
- **[CSS Overflow Module Level 4](https://www.w3.org/TR/css-overflow-4/) - Unsupported.**
  - CSS properties: `-webkit-line-clamp`, `block-ellipsis`, `continue`, `line-clamp`, `max-lines`, `overflow-clip-margin`,
    `overflow-clip-margin-block`, `overflow-clip-margin-block-end`, `overflow-clip-margin-block-start`,
    `overflow-clip-margin-bottom`, `overflow-clip-margin-inline`, `overflow-clip-margin-inline-end`,
    `overflow-clip-margin-inline-start`, `overflow-clip-margin-left`, `overflow-clip-margin-right`,
    `overflow-clip-margin-top`, `text-overflow`.
- **[CSS Overflow Module Level 5](https://www.w3.org/TR/css-overflow-5/) - Unsupported.**
  - CSS properties: `scroll-axis-lock`, `scroll-marker-group`, `scroll-target-group`.
- **[CSS Display Module Level 3](https://www.w3.org/TR/css-display-3/) - Unsupported.**
  - CSS properties: `display`, `order`, `visibility`.
- **[CSS Display Level 4](https://www.w3.org/TR/css-display-4/) - Unsupported.**
  - CSS properties: `display`, `order`, `reading-flow`, `reading-order`, `visibility`.
- **[CSS Inline Layout Module Level 3](https://www.w3.org/TR/css-inline-3/) - Unsupported.**
  - CSS properties: `alignment-baseline`, `baseline-shift`, `baseline-source`, `dominant-baseline`, `initial-letter`,
    `initial-letter-align`, `initial-letter-wrap`, `inline-sizing`, `line-fit-edge`, `line-height`, `text-box`,
    `text-box-edge`, `text-box-trim`, `vertical-align`.
- **[CSS Layout API Level 1](https://www.w3.org/TR/css-layout-api-1/) - Unsupported.**
- **[CSS Rhythmic Sizing Level 1](https://www.w3.org/TR/css-rhythm-1/) - Unsupported.**
  - CSS properties: `block-step`, `block-step-align`, `block-step-insert`, `block-step-round`, `block-step-size`, `line-height-step`.
- **[CSS Containment Module Level 1](https://www.w3.org/TR/css-contain-1/) - Unsupported.**
  - CSS properties: `contain`.
- **[CSS Containment Module Level 2](https://www.w3.org/TR/css-contain-2/) - Unsupported.**
  - CSS properties: `contain`, `content-visibility`.
- **[CSS Containment Module Level 3](https://www.w3.org/TR/css-contain-3/) - Unsupported.**
  - CSS properties: `container`, `container-name`, `container-type`.
- **[CSS Anchor Positioning](https://www.w3.org/TR/css-anchor-position-1/) - Unsupported.**
  - CSS properties: `align-self`, `anchor-name`, `anchor-scope`, `justify-self`, `position-anchor`, `position-area`, `position-try`,
    `position-try-fallbacks`, `position-try-order`, `position-visibility`.
- **[CSS Anchor Positioning Level 2](https://www.w3.org/TR/css-anchor-position-2/) - Unsupported.**

## Color, backgrounds, images, and effects

- **[CSS Backgrounds and Borders Level 3](https://www.w3.org/TR/css3-background/) - Unsupported.**
  - CSS properties: `background`, `background-attachment`, `background-clip`, ~~`background-color`~~, `background-image`,
    `background-origin`, `background-position`, `background-repeat`, `background-size`, `border`, `border-bottom`,
    `border-bottom-color`, `border-bottom-left-radius`, `border-bottom-right-radius`, `border-bottom-style`,
    `border-bottom-width`, `border-color`, `border-image`, `border-image-outset`, `border-image-repeat`,
    `border-image-slice`, `border-image-source`, `border-image-width`, `border-left`, `border-left-color`,
    `border-left-style`, `border-left-width`, ~~`border-radius`~~, `border-right`, `border-right-color`,
    `border-right-style`, `border-right-width`, `border-style`, `border-top`, `border-top-color`,
    `border-top-left-radius`, `border-top-right-radius`, `border-top-style`, `border-top-width`, `border-width`,
    `box-shadow`.
- **CSS Backgrounds Level 4 - Unsupported.**
  - CSS properties: `background`, `background-attachment`, `background-clip`, ~~`background-color`~~, `background-image`,
    `background-origin`, `background-position`, `background-position-block`, `background-position-inline`,
    `background-position-x`, `background-position-y`, `background-repeat`, `background-repeat-block`,
    `background-repeat-inline`, `background-repeat-x`, `background-repeat-y`, `background-size`, `background-tbd`.
- **CSS Borders and Box Decorations Level 4 - Unsupported.**
  - CSS properties: `border`, `border-block`, `border-block-clip`, `border-block-color`, `border-block-end`, `border-block-end-clip`,
    `border-block-end-color`, `border-block-end-radius`, `border-block-end-style`, `border-block-end-width`,
    `border-block-start`, `border-block-start-clip`, `border-block-start-color`, `border-block-start-radius`,
    `border-block-start-style`, `border-block-start-width`, `border-block-style`, `border-block-width`, `border-bottom`,
    `border-bottom-clip`, `border-bottom-color`, `border-bottom-left-radius`, `border-bottom-radius`,
    `border-bottom-right-radius`, `border-bottom-style`, `border-bottom-width`, `border-clip`, `border-color`,
    `border-end-end-radius`, `border-end-start-radius`, `border-image`, `border-image-outset`, `border-image-repeat`,
    `border-image-slice`, `border-image-source`, `border-image-width`, `border-inline`, `border-inline-clip`,
    `border-inline-color`, `border-inline-end`, `border-inline-end-clip`, `border-inline-end-color`,
    `border-inline-end-radius`, `border-inline-end-style`, `border-inline-end-width`, `border-inline-start`,
    `border-inline-start-clip`, `border-inline-start-color`, `border-inline-start-radius`, `border-inline-start-style`,
    `border-inline-start-width`, `border-inline-style`, `border-inline-width`, `border-left`, `border-left-clip`,
    `border-left-color`, `border-left-radius`, `border-left-style`, `border-left-width`, `border-limit`,
    ~~`border-radius`~~, `border-right`, `border-right-clip`, `border-right-color`, `border-right-radius`,
    `border-right-style`, `border-right-width`, `border-shape`, `border-start-end-radius`, `border-start-start-radius`,
    `border-style`, `border-top`, `border-top-clip`, `border-top-color`, `border-top-left-radius`, `border-top-radius`,
    `border-top-right-radius`, `border-top-style`, `border-top-width`, `border-width`, `box-shadow`, `box-shadow-blur`,
    `box-shadow-color`, `box-shadow-offset`, `box-shadow-position`, `box-shadow-spread`, `corner`, `corner-block-end`,
    `corner-block-end-shape`, `corner-block-start`, `corner-block-start-shape`, `corner-bottom`, `corner-bottom-left`,
    `corner-bottom-left-shape`, `corner-bottom-right`, `corner-bottom-right-shape`, `corner-bottom-shape`,
    `corner-end-end`, `corner-end-end-shape`, `corner-end-start`, `corner-end-start-shape`, `corner-inline-end`,
    `corner-inline-end-shape`, `corner-inline-start`, `corner-inline-start-shape`, `corner-left`, `corner-left-shape`,
    `corner-right`, `corner-right-shape`, `corner-shape`, `corner-start-end`, `corner-start-end-shape`,
    `corner-start-start`, `corner-start-start-shape`, `corner-top`, `corner-top-left`, `corner-top-left-shape`,
    `corner-top-right`, `corner-top-right-shape`, `corner-top-shape`.
- **[CSS Basic User Interface](https://www.w3.org/TR/css3-ui/) - Unsupported.**
  - CSS properties: `box-sizing`, `caret-color`, `cursor`, `outline`, `outline-color`, `outline-offset`, `outline-style`,
    `outline-width`, `resize`, `text-overflow`.
- **[CSS Color](https://www.w3.org/TR/css3-color/) - Unsupported.**
  - CSS properties: ~~`color`~~, `opacity`.
- **[CSS Color Level 4](https://www.w3.org/TR/css-color-4/) - Unsupported.**
  - CSS properties: ~~`color`~~, `opacity`.
- **[CSS Images Level 3](https://www.w3.org/TR/css3-images/) - Unsupported.**
  - CSS properties: `image-orientation`, `image-rendering`, `object-fit`, `object-position`.
- **[CSS Images Module Level 4](https://www.w3.org/TR/css4-images/) - Unsupported.**
  - CSS properties: `image-resolution`, `object-fit`.
- **[Compositing and Blending](https://www.w3.org/TR/compositing/) - Unsupported.**
  - CSS properties: `background-blend-mode`, `isolation`, `mix-blend-mode`.
- **[Filter Effects](https://www.w3.org/TR/filter-effects/) - Unsupported.**
  - CSS properties: `color-interpolation-filters`, `filter`, `flood-color`, `flood-opacity`, `lighting-color`.
- **[CSS Masking](https://www.w3.org/TR/css-masking/) - Unsupported.**
  - CSS properties: `clip`, `clip-path`, `clip-rule`, `mask`, `mask-border`, `mask-border-mode`, `mask-border-outset`,
    `mask-border-repeat`, `mask-border-slice`, `mask-border-source`, `mask-border-width`, `mask-clip`, `mask-composite`,
    `mask-image`, `mask-mode`, `mask-origin`, `mask-position`, `mask-repeat`, `mask-size`, `mask-type`.
- **[Geometry Interfaces Module Level 1](https://www.w3.org/TR/geometry-1/) - Unsupported.**
- **[CSS Basic User Interface Module Level 4](https://www.w3.org/TR/css-ui-4/) - Unsupported.**
  - CSS properties: `accent-color`, `appearance`, `caret`, `caret-animation`, `caret-color`, `caret-shape`, `cursor`, `interactivity`,
    `interest-delay`, `interest-delay-end`, `interest-delay-start`, `nav-down`, `nav-left`, `nav-right`, `nav-up`,
    `outline`, `outline-color`, `outline-offset`, `outline-style`, `outline-width`, `pointer-events`, `resize`,
    `user-select`, `window-drag`.
- **[CSS Fill and Stroke Module Level 3](https://www.w3.org/TR/fill-stroke-3/) - Unsupported.**
  - CSS properties: `fill`, `fill-break`, `fill-color`, `fill-image`, `fill-opacity`, `fill-origin`, `fill-position`, `fill-repeat`,
    `fill-rule`, `fill-size`, `stroke`, `stroke-align`, `stroke-break`, `stroke-color`, `stroke-dash-corner`,
    `stroke-dash-justify`, `stroke-dasharray`, `stroke-dashoffset`, `stroke-image`, `stroke-linecap`, `stroke-linejoin`,
    `stroke-miterlimit`, `stroke-opacity`, `stroke-origin`, `stroke-position`, `stroke-repeat`, `stroke-size`,
    `stroke-width`.
- **[CSS Color Adjust Level 1](https://www.w3.org/TR/css-color-adjust-1/) - Unsupported.**
  - CSS properties: `color-adjust`, `color-scheme`, `forced-color-adjust`, `print-color-adjust`.
- **[CSS Color Level 5](https://www.w3.org/TR/css-color-5/) - Unsupported.**
- **[CSS Color HDR Level 1](https://www.w3.org/TR/css-color-hdr-1/) - Unsupported.**
  - CSS properties: `dynamic-range-limit`.
- **[CSS Gap Decorations Level 1](https://www.w3.org/TR/css-gaps-1/) - Unsupported.**
  - CSS properties: `column-gap`, `column-rule`, `column-rule-break`, `column-rule-color`, `column-rule-inset`, `column-rule-inset-cap`,
    `column-rule-inset-cap-end`, `column-rule-inset-cap-start`, `column-rule-inset-end`, `column-rule-inset-junction`,
    `column-rule-inset-junction-end`, `column-rule-inset-junction-start`, `column-rule-inset-start`,
    `column-rule-style`, `column-rule-visibility-items`, `column-rule-width`, `gap`, `row-gap`, `row-rule`,
    `row-rule-break`, `row-rule-color`, `row-rule-inset`, `row-rule-inset-cap`, `row-rule-inset-cap-end`,
    `row-rule-inset-cap-start`, `row-rule-inset-end`, `row-rule-inset-junction`, `row-rule-inset-junction-end`,
    `row-rule-inset-junction-start`, `row-rule-inset-start`, `row-rule-style`, `row-rule-visibility-items`,
    `row-rule-width`, `rule`, `rule-break`, `rule-color`, `rule-inset`, `rule-inset-cap`, `rule-inset-end`,
    `rule-inset-junction`, `rule-inset-start`, `rule-overlap`, `rule-style`, `rule-visibility-items`, `rule-width`.
- **SVG - Unsupported.**

## Text, fonts, and generated content

- **[CSS Fonts Level 3](https://www.w3.org/TR/css3-fonts/) - Unsupported.**
  - CSS properties: `font`, ~~`font-family`~~, `font-feature-settings`, `font-kerning`, `font-size`, `font-size-adjust`, `font-stretch`,
    ~~`font-style`~~, `font-synthesis`, `font-variant`, `font-variant-caps`, `font-variant-east-asian`,
    `font-variant-ligatures`, `font-variant-numeric`, `font-variant-position`, ~~`font-weight`~~.
- **[CSS Generated Content for Paged Media](https://www.w3.org/TR/css3-gcpm/) - Unsupported.**
  - CSS properties: `footnote-display`, `footnote-policy`, `running`, `string-set`.
- **[CSS Hyperlink Presentation](https://www.w3.org/TR/css3-hyperlinks/) - Unsupported.**
- **[CSS Lists](https://www.w3.org/TR/css3-lists/) - Unsupported.**
  - CSS properties: `counter-increment`, `counter-reset`, `counter-set`, `list-style`, `list-style-image`, `list-style-position`,
    `list-style-type`, `marker-side`.
- **[CSS Ruby Annotation Layout](https://www.w3.org/TR/css3-ruby/) - Unsupported.**
  - CSS properties: `ruby-align`, `ruby-merge`, `ruby-overhang`, `ruby-position`.
- **[CSS Speech](https://www.w3.org/TR/css3-speech/) - Not planned.**
  - CSS properties: `cue`, `cue-after`, `cue-before`, `pause`, `pause-after`, `pause-before`, `rest`, `rest-after`, `rest-before`,
    `speak`, `speak-as`, `voice-balance`, `voice-duration`, `voice-family`, `voice-pitch`, `voice-range`, `voice-rate`,
    `voice-stress`, `voice-volume`.
- **[CSS Text](https://www.w3.org/TR/css3-text/) - Unsupported.**
  - CSS properties: `hanging-punctuation`, `hyphens`, `letter-spacing`, `line-break`, `overflow-wrap`, `tab-size`, `text-align`,
    `text-align-all`, `text-align-last`, `text-indent`, `text-justify`, `text-transform`, `white-space`, `word-break`,
    `word-spacing`.
- **[CSS Writing Modes](https://www.w3.org/TR/css3-writing-modes/) - Unsupported.**
  - CSS properties: `direction`, `glyph-orientation-vertical`, `text-combine-upright`, `text-orientation`, `unicode-bidi`,
    `writing-mode`.
- **[CSS Writing Modes Level 4](https://www.w3.org/TR/css-writing-modes-4/) - Unsupported.**
  - CSS properties: `direction`, `glyph-orientation-vertical`, `text-combine-upright`, `text-orientation`, `unicode-bidi`,
    `writing-mode`.
- **[CSS Line Grid](https://www.w3.org/TR/css-line-grid-1/) - Unsupported.**
  - CSS properties: `box-snap`, `line-grid`, `line-snap`.
- **[CSS Text Decoration Module Level 3](https://www.w3.org/TR/css-text-decor-3/) - Unsupported.**
  - CSS properties: `text-decoration`, `text-decoration-color`, `text-decoration-line`, `text-decoration-style`, `text-emphasis`,
    `text-emphasis-color`, `text-emphasis-position`, `text-emphasis-style`, `text-shadow`, `text-underline-position`.
- **[CSS Text Decoration Level 4](https://www.w3.org/TR/css-text-decor-4/) - Unsupported.**
  - CSS properties: `text-decoration`, `text-decoration-color`, `text-decoration-inset`, `text-decoration-line`, `text-decoration-skip`,
    `text-decoration-skip-box`, `text-decoration-skip-ink`, `text-decoration-skip-inset`, `text-decoration-skip-self`,
    `text-decoration-skip-spaces`, `text-decoration-style`, `text-decoration-thickness`, `text-emphasis`,
    `text-emphasis-color`, `text-emphasis-position`, `text-emphasis-skip`, `text-emphasis-style`, `text-shadow`,
    `text-underline-offset`, `text-underline-position`.
- **[CSS Font Loading Level 3](https://www.w3.org/TR/css-font-loading-3/) - Unsupported.**
- **[CSS Text Level 4](https://www.w3.org/TR/css-text-4/) - Unsupported.**
  - CSS properties: `hanging-punctuation`, `hyphenate-character`, `hyphenate-limit-chars`, `hyphenate-limit-last`,
    `hyphenate-limit-lines`, `hyphenate-limit-zone`, `hyphens`, `letter-spacing`, `line-break`, `line-padding`,
    `overflow-wrap`, `tab-size`, `text-align`, `text-align-all`, `text-align-last`, `text-autospace`,
    `text-group-align`, `text-indent`, `text-justify`, `text-spacing`, `text-spacing-trim`, `text-transform`,
    `text-wrap`, `text-wrap-mode`, `text-wrap-style`, `white-space`, `white-space-collapse`, `white-space-trim`,
    `word-break`, `word-space-transform`, `word-spacing`, `wrap-after`, `wrap-before`, `wrap-inside`.
- **[Fonts Level 4](https://www.w3.org/TR/css-fonts-4/) - Unsupported.**
  - CSS properties: `font`, ~~`font-family`~~, `font-feature-settings`, `font-kerning`, `font-language-override`, `font-optical-sizing`,
    `font-palette`, `font-size`, `font-size-adjust`, ~~`font-style`~~, `font-synthesis`, `font-synthesis-position`,
    `font-synthesis-small-caps`, `font-synthesis-style`, `font-synthesis-weight`, `font-variant`,
    `font-variant-alternates`, `font-variant-caps`, `font-variant-east-asian`, `font-variant-emoji`,
    `font-variant-ligatures`, `font-variant-numeric`, `font-variant-position`, `font-variation-settings`, ~~`font-weight`~~,
    `font-width`.
- **[Fonts Level 5](https://www.w3.org/TR/css-fonts-5/) - Unsupported.**
  - CSS properties: `font-size-adjust`.

## Animation and motion

- **[CSS Transforms Level 1](https://www.w3.org/TR/css-transforms-1/) - Unsupported.**
  - CSS properties: `transform`, `transform-box`, `transform-origin`.
- **CSS Transforms Level 2 - Unsupported.**
  - CSS properties: `backface-visibility`, `perspective`, `perspective-origin`, `rotate`, `scale`, `transform-style`, `translate`.
- **[CSS 3D Transformations Module](https://www.w3.org/TR/css3-3d-transforms/) - Unsupported.**
  - CSS properties: `backface-visibility`, `perspective`, `perspective-origin`, `rotate`, `scale`, `transform-style`, `translate`.
- **[CSS Transitions](https://www.w3.org/TR/css3-transitions/) - Unsupported.**
  - CSS properties: `transition`, `transition-delay`, `transition-duration`, `transition-property`, `transition-timing-function`.
- **[CSS Transitions Level 2](https://www.w3.org/TR/css-transitions-2/) - Unsupported.**
  - CSS properties: `transition-behavior`.
- **[CSS Animations Level 1](https://www.w3.org/TR/css-animations-1/) - Unsupported.**
  - CSS properties: `animation`, `animation-delay`, `animation-direction`, `animation-duration`, `animation-fill-mode`,
    `animation-iteration-count`, `animation-name`, `animation-play-state`, `animation-timing-function`.
- **[CSS Animations Level 2](https://www.w3.org/TR/css-animations-2/) - Unsupported.**
  - CSS properties: `animation-composition`, `animation-delay`, `animation-delay-end`, `animation-delay-start`, `animation-duration`,
    `animation-timeline`.
- **[Web Animations 1.0](https://www.w3.org/TR/web-animations/) - Unsupported.**
- **[Web Animations Level 2](https://www.w3.org/TR/web-animations-2/) - Unsupported.**
- **[Motion Path Module Level 1](https://www.w3.org/TR/motion-1/) - Unsupported.**
  - CSS properties: `offset`, `offset-anchor`, `offset-distance`, `offset-path`, `offset-position`, `offset-rotate`.
- **[CSS Easing Functions Level 1](https://www.w3.org/TR/css-easing-1/) - Unsupported.**
- **[CSS Easing Functions Level 2](https://www.w3.org/TR/css-easing-2/) - Unsupported.**
- **[CSS Animation Worklet API](https://www.w3.org/TR/css-animation-worklet-1/) - Unsupported.**
- **[CSS View Transitions Level 1](https://www.w3.org/TR/css-view-transitions-1/) - Unsupported.**
  - CSS properties: `view-transition-name`.
- **[CSS View Transitions Level 2](https://www.w3.org/TR/css-view-transitions-2/) - Unsupported.**
  - CSS properties: `view-transition-class`, `view-transition-group`, `view-transition-name`, `view-transition-scope`.
- **[Scroll-driven Animations](https://www.w3.org/TR/scroll-animations-1/) - Unsupported.**
  - CSS properties: `animation-range`, `animation-range-end`, `animation-range-start`, `scroll-timeline`, `scroll-timeline-axis`,
    `scroll-timeline-name`, `timeline-scope`, `view-timeline`, `view-timeline-axis`, `view-timeline-inset`,
    `view-timeline-name`.
- **[CSS Image Animation Level 1](https://www.w3.org/TR/css-image-animation-1/) - Unsupported.**
  - CSS properties: `image-animation`.

## Scrolling, viewport, and navigation

- **[CSS Viewport Level 1](https://www.w3.org/TR/css-viewport-1/) - Unsupported.**
  - CSS properties: `zoom`.
- **[Fullscreen](https://www.w3.org/TR/fullscreen/) - Unsupported.**
- **[CSS Scroll Snap Module Level 1](https://www.w3.org/TR/css-scroll-snap-1/) - Unsupported.**
  - CSS properties: `scroll-margin`, `scroll-margin-block`, `scroll-margin-block-end`, `scroll-margin-block-start`,
    `scroll-margin-bottom`, `scroll-margin-inline`, `scroll-margin-inline-end`, `scroll-margin-inline-start`,
    `scroll-margin-left`, `scroll-margin-right`, `scroll-margin-top`, `scroll-padding`, `scroll-padding-block`,
    `scroll-padding-block-end`, `scroll-padding-block-start`, `scroll-padding-bottom`, `scroll-padding-inline`,
    `scroll-padding-inline-end`, `scroll-padding-inline-start`, `scroll-padding-left`, `scroll-padding-right`,
    `scroll-padding-top`, `scroll-snap-align`, `scroll-snap-stop`, `scroll-snap-type`.
- **[CSS Scroll Snap Module Level 2](https://www.w3.org/TR/css-scroll-snap-2/) - Unsupported.**
  - CSS properties: `scroll-initial-target`, `scroll-start-target`.
- **[CSS Round Display Level 1](https://www.w3.org/TR/css-round-display-1/) - Unsupported.**
  - CSS properties: `border-boundary`, `shape-inside`.
- **[CSS Scrollbars Styling Level 1](https://www.w3.org/TR/css-scrollbars-1/) - Unsupported.**
  - CSS properties: `scrollbar-color`, `scrollbar-width`.
- **[CSS Spatial Navigation Level 1](https://www.w3.org/TR/css-nav-1/) - Unsupported.**
  - CSS properties: `spatial-navigation-action`, `spatial-navigation-contain`, `spatial-navigation-function`.
- **[CSS Overscroll Behavior Level 1](https://www.w3.org/TR/css-overscroll-1/) - Unsupported.**
  - CSS properties: `overscroll-behavior`, `overscroll-behavior-block`, `overscroll-behavior-inline`, `overscroll-behavior-x`,
    `overscroll-behavior-y`.
- **[CSS Scroll Anchoring Level 1](https://www.w3.org/TR/css-scroll-anchoring-1/) - Unsupported.**
  - CSS properties: `overflow-anchor`.

## CSS APIs and integrations

- **[CSS Object Model (CSSOM)](https://www.w3.org/TR/cssom/) - Unsupported.**
- **[CSSOM View Module](https://www.w3.org/TR/cssom-view/) - Unsupported.**
- **[CSS Painting API Level 1](https://www.w3.org/TR/css-paint-api-1/) - Unsupported.**
- **[CSS Properties and Values API Level 1](https://www.w3.org/TR/css-properties-values-api-1/) - Unsupported.**
- **[CSS Typed OM Level 1](https://www.w3.org/TR/css-typed-om-1/) - Unsupported.**
- **[Worklets Level 1](https://www.w3.org/TR/worklets-1/) - Unsupported.**
- **[Resize Observer](https://www.w3.org/TR/resize-observer-1/) - Unsupported.**

## Profiles, snapshots, and historical specifications

- **[Preview of CSS 2.2](https://www.w3.org/TR/CSS22/) - Unsupported.**
  - CSS properties: `background`, `background-attachment`, ~~`background-color`~~, `background-image`, `background-position`,
    `background-repeat`, `border`, `border-bottom`, `border-bottom-color`, `border-bottom-style`, `border-bottom-width`,
    `border-collapse`, `border-color`, `border-left`, `border-left-color`, `border-left-style`, `border-left-width`,
    `border-right`, `border-right-color`, `border-right-style`, `border-right-width`, `border-spacing`, `border-style`,
    `border-top`, `border-top-color`, `border-top-style`, `border-top-width`, `border-width`, `bottom`, `caption-side`,
    `clear`, `clip`, ~~`color`~~, `content`, `counter-increment`, `counter-reset`, `cursor`, `direction`, `display`,
    `empty-cells`, `float`, `font`, ~~`font-family`~~, `font-size`, ~~`font-style`~~, `font-variant`, ~~`font-weight`~~, ~~`height`~~,
    `left`, `letter-spacing`, `line-height`, `list-style`, `list-style-image`, `list-style-position`, `list-style-type`,
    ~~`margin`~~, ~~`margin-bottom`~~, ~~`margin-left`~~, ~~`margin-right`~~, ~~`margin-top`~~, `max-height`, `max-width`, `min-height`,
    `min-width`, `orphans`, `outline`, `outline-color`, `outline-style`, `outline-width`, `overflow`, ~~`padding`~~,
    ~~`padding-bottom`~~, ~~`padding-left`~~, ~~`padding-right`~~, ~~`padding-top`~~, `page-break-after`, `page-break-before`,
    `page-break-inside`, `position`, `property-name`, `quotes`, `right`, `table-layout`, `text-align`,
    `text-decoration`, `text-indent`, `text-transform`, `top`, `unicode-bidi`, `vertical-align`, `visibility`,
    `white-space`, `widows`, ~~`width`~~, `word-spacing`, `z-index`.
- **[CSS Snapshot 2007](https://www.w3.org/TR/css-beijing/) - Unsupported.**
- **[CSS Snapshot 2010](https://www.w3.org/TR/css-2010/) - Unsupported.**
- **[CSS Snapshot 2015](https://www.w3.org/TR/css-2015/) - Unsupported.**
- **[CSS Snapshot 2017](https://www.w3.org/TR/css-2017/) - Unsupported.**
- **[CSS Snapshot 2018](https://www.w3.org/TR/css-2018/) - Unsupported.**
- **[CSS Snapshot 2020](https://www.w3.org/TR/css-2020/) - Unsupported.**
- **[CSS Snapshot 2021](https://www.w3.org/TR/css-2021/) - Unsupported.**
- **[CSS Snapshot 2022](https://www.w3.org/TR/css-2022/) - Unsupported.**
- **[CSS Snapshot 2023](https://www.w3.org/TR/css-2023/) - Unsupported.**
- **[CSS Snapshot 2024](https://www.w3.org/TR/css-2024/) - Unsupported.**
- **[CSS Snapshot 2025](https://www.w3.org/TR/css-2025/) - Unsupported.**
- **[CSS Snapshot 2026](https://www.w3.org/TR/css-2026/) - Unsupported.**
- **CSS Aural Style Sheets - Not planned.**
- **CSS Extended Box Model - Not planned.**
- **[CSS Marquee](https://www.w3.org/TR/css3-marquee/) - Not planned.**
- **[CSS Introduction](https://www.w3.org/TR/css3-roadmap/) - Not planned.**
- **[CSS Presentation Levels](https://www.w3.org/TR/css3-preslev/) - Not planned.**
- **[CSS Reader Media Type](https://www.w3.org/TR/css3-reader/) - Not planned.**
- **[Grid Positioning](https://www.w3.org/TR/css3-grid/) - Not planned.**
- **[CSS Web Fonts](https://www.w3.org/TR/css3-webfonts/) - Not planned.**
- **[Behavioral Extensions to CSS](https://www.w3.org/TR/becss/) - Not planned.**
- **[CSS 2D Transformations Module](https://www.w3.org/TR/css3-2d-transforms/) - Not planned.**
- **[CSS Mobile Profile](https://www.w3.org/TR/css-mobile/) - Not planned.**
- **[CSS Print Profile](https://www.w3.org/TR/css-print/) - Not planned.**
- **[CSS TV Profile](https://www.w3.org/TR/css-tv) - Not planned.**
- **[CSS Level 1](https://www.w3.org/TR/CSS1/) - Not planned.**

## Additional property-index specifications and modules

These property-bearing specifications also appear in the W3C generated property index but do not have a separately matching feature heading above (or use a different title in the two indexes). This includes some editor-draft and legacy entries that overlap with newer modules.

### Core CSS, syntax, and cascading

- **Cascading Style Sheets Level 2 Revision 1 (CSS 2.1) Specification - Unsupported.**
  - CSS properties: `background`, `background-attachment`, ~~`background-color`~~, `background-image`, `background-position`,
    `background-repeat`, `border`, `border-bottom`, `border-bottom-color`, `border-bottom-style`, `border-bottom-width`,
    `border-collapse`, `border-color`, `border-left`, `border-left-color`, `border-left-style`, `border-left-width`,
    `border-right`, `border-right-color`, `border-right-style`, `border-right-width`, `border-spacing`, `border-style`,
    `border-top`, `border-top-color`, `border-top-style`, `border-top-width`, `border-width`, `bottom`, `caption-side`,
    `clear`, `clip`, ~~`color`~~, `content`, `counter-increment`, `counter-reset`, `cursor`, `direction`, `display`,
    `empty-cells`, `float`, `font`, ~~`font-family`~~, `font-size`, ~~`font-style`~~, `font-variant`, ~~`font-weight`~~, ~~`height`~~,
    `left`, `letter-spacing`, `line-height`, `list-style`, `list-style-image`, `list-style-position`, `list-style-type`,
    ~~`margin`~~, ~~`margin-bottom`~~, ~~`margin-left`~~, ~~`margin-right`~~, ~~`margin-top`~~, `max-height`, `max-width`, `min-height`,
    `min-width`, `orphans`, `outline`, `outline-color`, `outline-style`, `outline-width`, `overflow`, ~~`padding`~~,
    ~~`padding-bottom`~~, ~~`padding-left`~~, ~~`padding-right`~~, ~~`padding-top`~~, `page-break-after`, `page-break-before`,
    `page-break-inside`, `position`, `property-name`, `quotes`, `right`, `table-layout`, `text-align`,
    `text-decoration`, `text-indent`, `text-transform`, `top`, `unicode-bidi`, `vertical-align`, `visibility`,
    `white-space`, `widows`, ~~`width`~~, `word-spacing`, `z-index`.
- **CSS Form Control Styling Level 1 - Unsupported.**
  - CSS properties: `field-sizing`, `input-security`, `slider-orientation`.

### Color, backgrounds, images, and effects

- **CSS3 module: Border - Unsupported.**
  - CSS properties: `border`, `border-block`, `border-block-clip`, `border-block-color`, `border-block-end`, `border-block-end-clip`,
    `border-block-end-color`, `border-block-end-radius`, `border-block-end-style`, `border-block-end-width`,
    `border-block-start`, `border-block-start-clip`, `border-block-start-color`, `border-block-start-radius`,
    `border-block-start-style`, `border-block-start-width`, `border-block-style`, `border-block-width`, `border-bottom`,
    `border-bottom-clip`, `border-bottom-color`, `border-bottom-left-radius`, `border-bottom-radius`,
    `border-bottom-right-radius`, `border-bottom-style`, `border-bottom-width`, `border-clip`, `border-color`,
    `border-end-end-radius`, `border-end-start-radius`, `border-image`, `border-image-outset`, `border-image-repeat`,
    `border-image-slice`, `border-image-source`, `border-image-width`, `border-inline`, `border-inline-clip`,
    `border-inline-color`, `border-inline-end`, `border-inline-end-clip`, `border-inline-end-color`,
    `border-inline-end-radius`, `border-inline-end-style`, `border-inline-end-width`, `border-inline-start`,
    `border-inline-start-clip`, `border-inline-start-color`, `border-inline-start-radius`, `border-inline-start-style`,
    `border-inline-start-width`, `border-inline-style`, `border-inline-width`, `border-left`, `border-left-clip`,
    `border-left-color`, `border-left-radius`, `border-left-style`, `border-left-width`, `border-limit`,
    ~~`border-radius`~~, `border-right`, `border-right-clip`, `border-right-color`, `border-right-radius`,
    `border-right-style`, `border-right-width`, `border-shape`, `border-start-end-radius`, `border-start-start-radius`,
    `border-style`, `border-top`, `border-top-clip`, `border-top-color`, `border-top-left-radius`, `border-top-radius`,
    `border-top-right-radius`, `border-top-style`, `border-top-width`, `border-width`, `box-shadow`, `box-shadow-blur`,
    `box-shadow-color`, `box-shadow-offset`, `box-shadow-position`, `box-shadow-spread`, `corner`, `corner-block-end`,
    `corner-block-end-shape`, `corner-block-start`, `corner-block-start-shape`, `corner-bottom`, `corner-bottom-left`,
    `corner-bottom-left-shape`, `corner-bottom-right`, `corner-bottom-right-shape`, `corner-bottom-shape`,
    `corner-end-end`, `corner-end-end-shape`, `corner-end-start`, `corner-end-start-shape`, `corner-inline-end`,
    `corner-inline-end-shape`, `corner-inline-start`, `corner-inline-start-shape`, `corner-left`, `corner-left-shape`,
    `corner-right`, `corner-right-shape`, `corner-shape`, `corner-start-end`, `corner-start-end-shape`,
    `corner-start-start`, `corner-start-start-shape`, `corner-top`, `corner-top-left`, `corner-top-left-shape`,
    `corner-top-right`, `corner-top-right-shape`, `corner-top-shape`.
- **SVG Markers - Unsupported.**
  - CSS properties: `marker`, `marker-end`, `marker-knockout-left`, `marker-knockout-right`, `marker-mid`, `marker-pattern`,
    `marker-segment`, `marker-start`.
- **SVG Strokes - Unsupported.**
  - CSS properties: `stroke`, `stroke-alignment`, `stroke-dashadjust`, `stroke-dasharray`, `stroke-dashcorner`, `stroke-dashoffset`,
    `stroke-linecap`, `stroke-linejoin`, `stroke-miterlimit`, `stroke-opacity`, `stroke-width`.
