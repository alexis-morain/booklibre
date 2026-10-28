---
name: booklibre
description: A quiet white page, like a well-kept notebook, where four pale tags say what each thing is and the cream paper of the book is the only warm object on screen.
colors:
  ground: "#ffffff"
  sidebar: "#fbfbfa"
  wash: "#f7f7f5"
  table: "#f1f0ec"
  line: "#e9e9e7"
  ink: "#37352f"
  ink-muted: "#6b6a65"
  field-border: "#8e8c86"
  action: "#191918"
  action-hover: "#3b3a36"
  action-tint: "#efefed"
  on-action: "#ffffff"
  link: "#1d63c9"
  error: "#c4332b"
  error-tint: "#fbeceb"
  chap: "#f6e3d8"
  chap-ink: "#7a3313"
  chap-rule: "#b04a1f"
  qual: "#fbf0d4"
  qual-ink: "#5c4210"
  main: "#e1edf6"
  main-ink: "#1d4466"
  main-rule: "#3a78ad"
  pret: "#e3f0e4"
  pret-ink: "#245133"
  gris: "#efefed"
  gris-ink: "#55534e"
  paper: "#f6f2e9"
  paper-ink: "#3a342c"
  paper-ink-soft: "#6b6459"
  link-paper: "#1d63c9"
  paper-warn: "#8a3a14"
  dark-ground: "#191918"
  dark-sidebar: "#202020"
  dark-wash: "#252524"
  dark-table: "#141413"
  dark-line: "#2f2f2d"
  dark-ink: "#e6e5e1"
  dark-ink-muted: "#9b9a95"
  dark-field-border: "#7a7974"
  dark-action: "#e6e5e1"
  dark-on-action: "#191918"
  dark-link: "#5a9ee6"
  dark-error: "#ef6b60"
typography:
  display:
    fontFamily: "ui-sans-serif, -apple-system, BlinkMacSystemFont, Segoe UI, Helvetica, Arial, sans-serif"
    fontSize: "28px"
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "-0.02em"
  title:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "18px"
    fontWeight: 700
    lineHeight: 1.2
  body:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.45
  body-small:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.45
  tag:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "12px"
    fontWeight: 600
    lineHeight: 1.4
  figures:
    fontFamily: "{typography.display.fontFamily}"
    fontFeature: "tnum"
  book:
    fontFamily: "colophon-album, Source Sans 3, sans-serif"
    fontWeight: 400
rounded:
  tag: "4px"
  control: "6px"
  panel: "10px"
  paper: "0px"
spacing:
  space-1: "4px"
  space-2: "8px"
  space-3: "12px"
  space-4: "16px"
  space-5: "24px"
  space-6: "32px"
components:
  button-primary:
    backgroundColor: "{colors.action}"
    textColor: "{colors.on-action}"
    rounded: "{rounded.control}"
    height: "34px"
  button-primary-hover:
    backgroundColor: "{colors.action-hover}"
  button-secondary:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.control}"
  input:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
  tag-chap:
    backgroundColor: "{colors.chap}"
    textColor: "{colors.chap-ink}"
    rounded: "{rounded.tag}"
  tag-qual:
    backgroundColor: "{colors.qual}"
    textColor: "{colors.qual-ink}"
    rounded: "{rounded.tag}"
  tag-main:
    backgroundColor: "{colors.main}"
    textColor: "{colors.main-ink}"
    rounded: "{rounded.tag}"
  tag-pret:
    backgroundColor: "{colors.pret}"
    textColor: "{colors.pret-ink}"
    rounded: "{rounded.tag}"
  panel:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.ink}"
    rounded: "{rounded.panel}"
  sidebar:
    backgroundColor: "{colors.sidebar}"
    textColor: "{colors.ink-muted}"
  sidebar-item-active:
    backgroundColor: "{colors.action-tint}"
    textColor: "{colors.ink}"
  paper:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.paper-ink}"
    rounded: "{rounded.paper}"
---

# Design System: Booklibre

## Overview

**North Star: "The Quiet Page"**

The desktop app looks like a well-kept notebook: white, warm grey ink, one hairline, nothing that asks for attention except the one thing on screen. It is the same family as tri-offres (Notion, OpenSEO, Twenty) with one difference that is the product itself: the app makes a book, so **the cream paper of the spreads is the only warm object on the screen**. Photographs carry the colour. The chrome gets out of their way.

The structure is a page of a database: a sidebar of pages (the four views, the chapters as a tree), a breadcrumb on top, and in the book view the sheet on its table with a properties panel beside it, like a Notion page.

**Key Characteristics:**
- White ground, warm grey ink (`#37352f`), one hairline (`#e9e9e7`); dark follows the system and stays warm.
- Near-black actions, one per screen. Blue only for focus, links and selection.
- Terracotta lives only in the chapter (tag, tree mark) and the brand mark.
- Four pale tags, one meaning each. The platform's own sans, no webfont for the chrome.
- Three corners, each with one job; the paper stays square.

## Colors

Neutral, warm, low-contrast surfaces; contrast is spent on text, on the single primary action and on the paper.

### Neutrals
- **Ground** (`ground`): the page, fields, panels, menus. **Sidebar** (`sidebar`) one step warmer. **Wash** (`wash`): the quiet second surface. **Table** (`table`): the desk the sheet lies on, the only surface with the sheet's shadow.
- **Hairline** (`line`): every 1px divider. **Ink** (`ink`) and **Muted ink** (`ink-muted`) for text. **Field border** (`field-border`): the boundary of a field, held at 3:1.

### Action
- **Near Black** (`action`): the primary button, checked boxes, progress. Hover lightens to `action-hover`; `action-tint` is the selected row, the hovered ghost, the active sidebar entry. In the dark theme the action is the light ink and `on-action` the dark ground.
- **Link Blue** (`link`): the focus ring (2px), links, and the selection on the paper. What is drawn on the paper keeps `link-paper` and `paper-warn` in both themes, because paper has no theme.

### Error
- **Red** (`error`, `error-tint`): errors and destruction only, never a tag. The one red menu entry is Supprimer, alone in its group.

### Tags (the four meanings)
- **Chapter** (`chap`, terracotta): the chapter a spread opens or belongs to. **To check** (`qual`, yellow): what will print badly, a preflight that warns. **By hand** (`main`, blue): edited, pinned, ordered. **Ready** (`pret`, green): verified. Grey for the rest (`gris`). A tag is a pale fill with a dark ink; the terracotta and the blue also keep a mid-tone for the small marks that carry them.

### Named Rules
**The One Tag Per Meaning Rule.** A tag colour is never decoration. A new surface reuses a colour only for the same meaning.

**The Paper Is Warm Rule.** The cream (`paper`, `#f6f2e9`) does not change with the theme. Nothing else on screen is that warm.

**The Tested Pair Rule.** `couleurs-chrome.test.ts` reads `styles.css` and holds every pair: 4.5:1 for text, 3:1 for a field boundary, in both themes. A new pair is added to the test before it ships.

## Typography

**Family:** the platform's sans (`ui-sans-serif`, San Francisco, Segoe UI) for the whole chrome; nothing is downloaded. The text of the book (captions, blocks, the cover on screen) keeps `--font-book`, the face the PDF embeds, Source Sans 3 by default: what the paper shows is what prints, and the chrome never pretends to be it.

### Hierarchy
- **Display** (700, 28px, -0.02em): the welcome title. **Title** (700, 18px): the properties heading, dialogs. **Body** (400, 14px). **Body small** (400, 13px): panels, hints. **Tag** (600, 12px). Nothing is set under 12px in the chrome. Figures the system reports (counts, pages, sizes) use tabular figures (`.typed`).

## Layout

The window is a sidebar (240px) and a column. The column is a 48px top bar, the view, then the foot (page ruler, status line, quality). The book view puts the sheet on the table at the left and a 290px properties panel at the right. Below 1180px of window the sidebar folds itself; `⌘\` toggles it, and the choice, once made, holds.

**The One Primary Rule.** A screen has one near-black button. The top bar's « Exporter le PDF » leads to Envoi, and disappears there, where Envoi's own button is the primary.

## Elevation & Depth

Flat by default. Two shadows exist: the layered menu shadow of Notion (`--shadow-float`) for what floats (menus, popovers, a dragged spread), and the sheet's shadow (`--shadow-sheet`) for a spread lying on the table. Panels, cards and buttons never cast one.

## Shapes

Three corners: 4px for tags and small marks, 6px for controls (buttons, fields, menu rows, segments), 10px for panels (dialogs, menus, drawers, cards, encarts). The paper (spreads, pages, thumbnails of spreads, photo cells, the cover) stays square, like a trimmed page. Borders are 1px; the only 2px lines are the focus ring, the selection outline on the paper, and a selected spread in a grid (ink).

The brand mark is two small squares, the book's ink beside its terracotta, then the wordmark in 700. Icons are inline SVG on a 24-unit grid, stroke in `currentColor`, always decorative.

## Components

- **Buttons:** primary (near black, `.cta`), secondary (white, hairline, `.btn-second`), ghost (muted, tint on hover, `.link`). A disabled control lowers its opacity.
- **Fields:** white, `field-border`, 6px, a 2px blue ring on focus.
- **Menus and popovers:** white, 10px, `--shadow-float`, rows of 6px; shortcuts at the right in muted ink; a hairline between groups; Supprimer alone at the end, in red.
- **Encarts:** a pale ground (`qual`, `error-tint`, `wash`), 6px, no border and never a coloured edge; gestures are underlined words at the end.
- **Sidebar:** the album (cover thumbnail, name, format and spreads), the four views with counts and their shortcuts, the chapters as a tree with the terracotta mark, then at the foot the font and New album.
- **Properties panel:** label and value rows (chapter tag, disposition picker, moment, state, quality), a hairline, the chosen photo, its quality encart with its gestures, the adjustments as stacked sliders.
- **Light table:** views as tabs (Toutes, À vérifier, Éditées); the chapter tag under the spread that opens it; quality and edited tags on the right; the selected spread is outlined in ink, 2px.

### Motion
Controls ease over 160ms on `cubic-bezier(0.16, 1, 0.3, 1)`, on opacity, colour and transform, never on layout. Under `prefers-reduced-motion` everything collapses.

## Do's and Don'ts

### Do:
- **Do** keep the chrome white and the paper cream; let the photographs carry the colour.
- **Do** use near black for the one primary action and blue for focus and selection only.
- **Do** name chapters, quality and hand work with their tag, and nothing else.
- **Do** add every new colour pairing to `couleurs-chrome.test.ts`.
- **Do** write French copy in the vouvoiement, with curly apostrophes (’) and French quotes (« »).

### Don't:
- **Don't** put terracotta on an action, a selection or a focus ring.
- **Don't** put a coloured border on one side of a panel or an encart.
- **Don't** add a shadow to a panel, a card or a button.
- **Don't** round the paper.
- **Don't** load a font for the chrome.
- **Don't** use an em dash anywhere in the interface copy.
