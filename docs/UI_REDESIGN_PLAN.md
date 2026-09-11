# UI Redesign Plan

This branch restarts the visual redesign from the current `main` UI baseline.

The previous redesign attempt was intentionally discarded from this branch and preserved in:

- `backup/ui-redesign-i18n-before-restart`

The new process is intentionally staged. We do **not** redesign all pages at once.

## Product character

DLSS5oneclick is a local Windows graphics utility, not a website, SaaS dashboard, launcher, or marketing app.

The target feeling is:

- compact Windows desktop utility;
- dark and technical;
- visually calm;
- dense enough to feel native to desktop;
- clear primary action hierarchy;
- no web-style hero sections, KPI dashboards, giant section containers, or excessive empty space.

## Design principles

1. Preserve the current product workflow and business logic.
2. Improve hierarchy before changing colors.
3. Prefer direct section headers over container-inside-container layouts.
4. Use width deliberately; avoid narrow centered web columns.
5. Keep page-level information compact.
6. Treat game cards as the primary visual component of the Games page.
7. Keep technical metadata visible but secondary.
8. One redesign phase = one visual problem.

## Baseline

The branch starts from the current `main` commit and therefore keeps the original interface structure and behavior.

No i18n framework, alternate component system, or redesigned theme is introduced in Phase 1.

---

# Phase 1 — Games wireframe

Goal: improve only the macro-layout of the Games page while keeping the native theme and existing card rendering as much as possible.

No Setup, Settings, About, localization, or global theme refactor in this phase.

## Target desktop geometry

Reference viewport: `1600 × 900`.

Minimum supported viewport remains whatever the current application already supports.

Target content behavior:

- page uses most of the available desktop width;
- outer page padding: about `24–32 px`;
- no narrow `900 px` web column on a wide desktop window;
- no page-wide KPI/hero card;
- no large empty framed groups.

## Games page hierarchy

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ APP HEADER                                                                 │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│ Games                                             3 games · 2 configured   │
│ Manage DLSS 5 installation and settings                                  │
│                                                                            │
│ [ Search games................................ ] [Add game] [Folder] [Scan]│
│                                                                            │
│ INSTALLED BY THIS TOOL                                              2      │
│                                                                            │
│ ┌──────────────────────────────┐  ┌──────────────────────────────┐         │
│ │ poster  Lies of P            │  │ poster  Mass Effect LE      │         │
│ │         path                 │  │         path                 │         │
│ │         status metadata      │  │         status metadata      │         │
│ │         primary action       │  │         primary action       │         │
│ └──────────────────────────────┘  └──────────────────────────────┘         │
│                                                                            │
│ STEAM                                                               1     │
│                                                                            │
│ ┌──────────────────────────────┐                                           │
│ │ poster  Blender              │                                           │
│ │         Vulkan               │                                           │
│ │         not installed        │                                           │
│ │         Install DLSS 5       │                                           │
│ └──────────────────────────────┘                                           │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

## Page header

Do not use a framed hero card.

Use a simple desktop page heading:

- title: `Games` / later localized;
- small secondary summary on the same horizontal line or directly below;
- example summary: `3 games · 2 configured`;
- subtitle is optional and should stay short.

The title block should occupy little vertical space.

## Toolbar

One row directly below the page heading.

- search receives the flexible width;
- actions stay compact;
- `Scan` is the primary action;
- `Add game` and `Add folder` are secondary;
- no surrounding card.

Target control height: roughly `34–38 px`.

## Sections

Sections are separated by whitespace and a section header only.

Do **not** wrap the entire section in another full-width card.

Header example:

```text
INSTALLED BY THIS TOOL                                     2
```

Optional secondary status can appear at the right, but should remain quiet.

## Game card direction

The existing poster-only grid is compact but hides too much information.

The eventual target is a horizontal utility card:

```text
┌────────────────────────────────────────────┐
│ ┌─────────┐  Lies of P                     │
│ │ poster  │  E:\Steam\...\Lies of P        │
│ │         │  [Installed] [Native] [DLSS 5] │
│ │         │                                │
│ └─────────┘  [Open / Configure]            │
└────────────────────────────────────────────┘
```

However, Phase 1 should not fully restyle the card yet. First establish page rhythm and available width.

## Visual density

Desktop target:

- page title: `20–22 px` visual size;
- section title: `12–13 px semibold`;
- body: `12–13 px`;
- technical/path text: `10.5–11.5 px`;
- section gap: `20–24 px`;
- card gap: `14–16 px`.

The app should feel compact but not miniaturized.

## Explicit anti-patterns

Do not introduce:

- KPI dashboard cards;
- a giant Games hero panel;
- nested full-width section cards;
- a sidebar;
- web-style breadcrumbs;
- excessive rounded panels;
- giant headings;
- decorative gradients;
- neon/glow effects;
- marketing UI;
- a global component framework before the layout is approved.

---

# Planned phases

## Phase 1
Games macro-layout only.

## Phase 2
Game card component.

## Phase 3
Header/navigation polish and shared spacing rules.

## Phase 4
Setup information architecture.

## Phase 5
Settings and About.

## Phase 6
RU/EN localization after the visual structure is stable.

## Phase 7
Responsive and accessibility pass.

---

# Acceptance rule

A phase is not considered complete because the code compiles.

It is complete only after:

1. the screen is visually reviewed at desktop size;
2. spacing and hierarchy look intentional;
3. no product behavior has regressed;
4. only then do we continue to the next phase.
