# JieqiBox UI redesign — design plan

## Subject
Jieqi (揭棋 / "revealing chess") analysis workbench. Audience: serious players
and engine developers. Page job: read the board + engine analysis at a glance,
drive the engine without friction. This is a **tool / terminal**, not a
consumer app — the reference points are Lichess analysis board and modern data
terminals, not Material Design demos.

## What makes it feel "last-century" today
- Stock Vuetify Material blue (#1976D2) primary — the most generic default.
- Rainbow toolbar: teal/amber/lime/orange/cyan/deep-purple/indigo/deep-orange
  icons crammed side by side.
- Saturated candy buttons in the sidebar (yellow/red/teal/cyan).
- Heavy drop shadows, default radii, no type hierarchy.

## Design tokens

### Color — "Graphite & Cinnabar"
Derived from the subject: Jieqi is red vs black with hidden pieces. The one
accent is **cinnabar red** (朱砂), the traditional ink of the red 帥 — culturally
true to the game, and NOT the terracotta/clay (#D97757) AI tell.

Light theme:
- `--surface-base`   #F1F0EC  app background (warm paper-gray, not cream #F4F1EA)
- `--surface-1`      #FFFFFF  panels
- `--surface-2`      #E7E5DF  insets / pool cells
- `--ink`            #1C1B19  primary text (near-black warm)
- `--ink-soft`       #6B6862  secondary text
- `--accent`         #C0362C  cinnabar (primary actions, focus)
- `--line`           #D8D5CD  hairline borders

Dark theme (primary target — tools live in dark):
- `--surface-base`   #16181D  graphite
- `--surface-1`      #1E2127  panels
- `--surface-2`      #262A31  insets
- `--ink`            #E8E6E1
- `--ink-soft`       #8B9099
- `--accent`         #E4483B  cinnabar, brighter for dark
- `--line`           #2E333B

Semantic (both themes, used sparingly): success #3F9B5B, danger = accent family,
warning #C98A2B. Black-side motif: #2B2E35 chips.

### Type
- UI / display: system UI sans stack tuned like Inter — 
  `'Inter', -apple-system, 'Segoe UI', system-ui, sans-serif`.
  Weights: 600 for titles/labels, 500 for buttons, 400 body.
- **Data / monospace**: `'JetBrains Mono', 'SF Mono', ui-monospace, monospace`
  for scores, depth, nodes, NPS, FEN, engine log. This is the soul of an
  analysis tool — every number is data and should align in a mono grid.
- Type scale: title 15px/600, label 12px/600 uppercase tracked, body 13px,
  data 13–28px mono.

### Layout
Keep the two-column workbench (board left, analysis rail right). It's correct
for the task — don't reinvent. The work is in *density and hierarchy*: tighten
the toolbar into a single quiet icon strip, group the sidebar controls into
clean segmented rows, turn the capture pool into a precise data grid.

### Signature
**The capture/reveal pool as an instrument panel.** Jieqi's identity is hidden
pieces being revealed; the "暗子池" is the one screen element unique to this
game. Treat it as the hero: a tight monospace grid of red/black piece chips with
count(revealed) readouts, cinnabar for red side, graphite for black, hairline
separators — like a live inventory gauge. Everything else stays quiet so this
reads as the memorable element.

## Restraint
Spend boldness on the accent (cinnabar) + the pool. Kill all other named colors:
map every rainbow button to accent / neutral-surface / semantic. Flatten
shadows to hairline borders + one soft elevation shadow. Consistent radius scale
(6/8/10). Respect reduced-motion; keep visible focus rings in cinnabar.

## Build approach (token-driven, low-risk)
1. Retune Vuetify theme in `main.ts` — light + dark palettes above. This
   propagates through all `rgb(var(--v-theme-*))` usages app-wide.
2. Add a global stylesheet (`src/styles/theme.scss`) imported in main: CSS vars,
   button/card/panel refinement, mono font for data, focus rings, scrollbars.
   Load Inter + JetBrains Mono via fontsource or CSS.
3. Replace rainbow `color="..."` props in TopToolbar + AnalysisSidebar with a
   restrained set (accent / neutral / success / warning / error).
4. Refine the capture pool grid styling as the signature.

## Tried / learnings
- (first pass) baseline captured; theme is the single highest-leverage lever.
