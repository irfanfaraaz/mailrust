---
name: Mailrust
description: Field notes for your mail room. A parchment-and-navy-ink dashboard for newsletters and the sending API.
colors:
  parchment: "#f5f4ed"
  paper: "#fbfaf5"
  sidebar-parchment: "#efede4"
  sidebar-active: "#e4e2d7"
  fog: "#e9e7dd"
  muted-paper: "#ebe9e0"
  ink: "#1a2030"
  ink-soft: "#5b6070"
  carrier-navy: "#1f3d7a"
  on-navy: "#ffffff"
  rule: "#dcd9cd"
  field-rule: "#c8c5b8"
  good-green: "#1d7a43"
  wait-amber: "#a35a0a"
  danger-red: "#c42b1c"
  chart-violet: "#6b55b8"
  tab-newsletter: "#c4632f"
  tab-api: "#1b8079"
  tab-settings: "#6b55b8"
typography:
  heading:
    fontFamily: "Source Serif 4, ui-serif, Georgia, serif"
    fontSize: "1.875rem"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "-0.012em"
  title:
    fontFamily: "Source Serif 4, ui-serif, Georgia, serif"
    fontSize: "1.125rem"
    fontWeight: 600
    lineHeight: 1.375
    letterSpacing: "-0.012em"
  body:
    fontFamily: "Geist, ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.5
  figure:
    fontFamily: "Geist Mono, ui-monospace, monospace"
    fontSize: "1.875rem"
    fontWeight: 600
    letterSpacing: "-0.025em"
  label:
    fontFamily: "Geist, ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.6875rem"
    fontWeight: 500
    letterSpacing: "0.08em"
  stamp:
    fontFamily: "Geist Mono, ui-monospace, monospace"
    fontSize: "0.6875rem"
    fontWeight: 700
    letterSpacing: "0.08em"
rounded:
  base: "8px"
  sm: "4.8px"
  md: "6.4px"
  xl: "11.2px"
  sketch-card: "40px 10px 36px 10px / 10px 36px 10px 40px"
  sketch-panel: "28px 8px 24px 8px / 8px 24px 8px 28px"
  sketch-stat: "255px 14px 225px 14px / 14px 225px 14px 255px"
  stamp: "2px"
spacing:
  card-pad: "20px"
  card-pad-sm: "12px"
  page-x: "40px"
  page-y: "36px"
  page-x-phone: "16px"
  section-gap: "32px"
components:
  button-primary:
    backgroundColor: "{colors.carrier-navy}"
    textColor: "{colors.on-navy}"
    rounded: "{rounded.sketch-card}"
    height: "36px"
    padding: "0 14px"
  button-outline:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    height: "36px"
    padding: "0 14px"
  button-outline-hover:
    backgroundColor: "{colors.muted-paper}"
  card:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.sketch-card}"
    padding: "20px"
  input:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    height: "36px"
    padding: "4px 12px"
  stamp:
    textColor: "{colors.good-green}"
    typography: "{typography.stamp}"
    rounded: "{rounded.stamp}"
    padding: "1px 6px"
---

# Design System: Mailrust

## Overview

**Creative North Star: "The Field Notebook"**

The dashboard reads like a writer's notebook kept on parchment with navy ink, not a grey SaaS admin. The ground is warm parchment, cards are slightly lighter paper, text is ink-dark, and a single Carrier-navy ink does all the acting. Cards are drawn by hand: a 2px ink border with uneven corners. Status is a rubber stamp. Titles are serif; figures are mono.

Density is comfortable and operate-mode: one 248px sidebar, one content column, generous gaps between sections. The look was chosen by the user from a picker (Kami start, Parchment base, Carrier navy accent, Source Serif 4 + Geist, colour tab per sidebar group, Sketch cards). It explicitly refuses flat white cards, pill badges and a neutral grey sidebar.

Light is the only shipped theme. A full `.dark` token set exists in the stylesheet (navy lifts to #8aa8e6, paper to #1c1f26) but no theme toggle ships yet, so dark is unverified in the UI.

**Key Characteristics:**
- Parchment ground, paper cards, ink text, one navy accent.
- Hand-drawn 2px ink-bordered cards with uneven corners.
- Serif titles, Geist body, Geist Mono for every figure.
- Rubber-stamp status labels, tilted 2 degrees.
- Tracked uppercase small labels.
- A colour tab per sidebar group, kept off the accent's turf.

## Colors

A warm, near-monochrome parchment-and-ink palette with one deep navy accent; green, amber and red are reserved for meaning.

### Primary
- **Carrier Navy** (#1f3d7a): the only action colour. Primary buttons, links, focus ring, selected chart series, native controls (checkbox, radio, progress via accent-color), editor links and quotes. In dark it lifts to a pale periwinkle (#8aa8e6).

### Secondary
- **Tab Terracotta** (#c4632f), **Tab Teal** (#1b8079), **Tab Violet** (#6b55b8): one per sidebar group (Newsletter, Sending API, Settings). They mark the active item's left inset bar and tint the group label (55% mixed into ink). Nothing else uses them.

### Neutral
- **Parchment** (#f5f4ed): page ground.
- **Paper** (#fbfaf5): cards, popovers, inputs.
- **Sidebar Parchment** (#efede4) with active row **#e4e2d7**: the shell's rail.
- **Fog** (#e9e7dd) and **Muted Paper** (#ebe9e0): secondary buttons, hover fills, merge-tag chips.
- **Ink** (#1a2030): text and the sketch border.
- **Ink Soft** (#5b6070): secondary text, labels, chevron.
- **Rule** (#dcd9cd) and **Field Rule** (#c8c5b8): dividers and input borders.

### Status
- **Good Green** (#1d7a43): delivered, sent, verified, subscribed, done.
- **Wait Amber** (#a35a0a): pending, delayed, paused, the one-time-secret card border.
- **Danger Red** (#c42b1c): bounced, failed, destructive, paused sending.

### Named Rules
**The One Ink Rule.** Navy is the only accent for actions and links. Green, amber and red mean good, waiting and danger and nothing else.

**The Quiet Tab Rule.** Group tab colours appear only in the sidebar (label tint and active inset bar). They never fill a surface or a button.

## Typography

**Display Font:** Source Serif 4 (with ui-serif, Georgia)
**Body Font:** Geist (with system sans)
**Label/Mono Font:** Geist Mono

**Character:** A bookish serif for titles over a clean neutral sans, with mono for anything countable. The pairing reads like a typeset notebook with a ledger in it.

### Hierarchy
- **Heading** (600, 1.875rem, 1.25): page titles via PageHeader; all h1-h3 inherit serif with -0.012em tracking.
- **Title** (600, 1.125rem to 1.25rem, 1.375): card titles and the latest-issue name.
- **Body** (400, 0.875rem, 1.5): everything else; description lines cap at prose width. Editor prose runs 1.6 line height.
- **Figure** (Geist Mono 600, 1.875rem stat tiles, 1.25rem inline): every number, via the tabular-nums class (which sets the mono face).
- **Label** (500, 0.6875rem, +0.08em, UPPERCASE): stat tile captions, table headers, sidebar group names.

### Named Rules
**The Ledger Rule.** Every figure is Geist Mono; every title is Source Serif 4. Do not set a number in the sans or a title in the sans.

## Layout

Operate-mode shell: a two-column grid, 248px sticky sidebar then a fluid content column. On phones the sidebar stacks above content. Content padding is 40px by 36px on desktop and 16px by 24px on phones; page sections sit 32px apart. Stat tiles run four across on desktop and two per row on phones. Tap targets rise to 44px (min-h-11 on buttons, h-11 inputs) below the md breakpoint.

The sidebar order is: account switcher, full-width "New issue" button, grouped nav with icons, then the sending-health strip pinned to the bottom. The page header puts the title and one context line left, actions right.

## Elevation & Depth

Depth is drawn, not blurred. A sketch card carries a short hard offset in translucent ink (3px 4px 0, ink at 16%), and the primary button a smaller one (2px 2px 0). Everything else (inputs, secondary and ghost buttons, health strip, tables rows) is flat; hover is a tonal fill shift. Nothing uses a soft blurred shadow.

### Shadow Vocabulary
- **Ink Shadow, card** (`box-shadow: 3px 4px 0 var(--ink-shadow)`): every Card and the table frame.
- **Ink Shadow, button** (`box-shadow: 2px 2px 0 var(--ink-shadow)`): the primary button only.

### Named Rules
**The Drawn Depth Rule.** Elevation is a hand-drawn offset in ink at low opacity, only on sketch surfaces. Never a blurred drop shadow.

## Shapes

Soft corners everywhere except the sketch devices. Base radius is 8px (inputs 6.4px, small buttons 4.8px to 12px). Sketch surfaces use deliberately uneven corners with a 2px solid ink border in three strengths: card (40/10/36/10 over 10/36/10/40), panel for tables and the editor (28/8/24/8, a calmer wobble), and stat tile (a near-pill 255/14/225/14 wobble). Buttons that are sketched use a tighter 225/12/255/12 wobble. A warning variant swaps the ink border for amber. Stamps are the opposite: a 2px-radius rectangle, 1.5px border plus an outer 1px hairline at 40%, tilted -2 degrees.

## Components

### Buttons
- **Shape:** sketched wobble corners (primary and outline), 2px ink border.
- **Primary:** Carrier Navy fill, white text, 36px high, 14px side padding, small ink offset shadow; hover is navy at 90%.
- **Outline:** paper fill, ink border, hover Muted Paper.
- **Secondary / Ghost / Destructive / Link:** flat, no border; secondary is Fog, destructive is red tint, link is navy underline on hover.
- **Focus:** 3px navy ring at 50%. Active presses down 1px.

### Cards / Containers
- **Corner Style:** sketch card; stat tiles use the stat wobble, tables and editor the panel wobble.
- **Background:** Paper. **Border:** 2px ink. **Shadow:** Ink Shadow, card.
- **Internal Padding:** 20px (12px for the small size).

### Inputs / Fields
- **Style:** 1.5px Field Rule border on Paper, 6.4px radius, 36px high (44px on phones).
- **Focus:** border and 3px ring in navy. **Error:** red border and ring. **Disabled:** 50% opacity.
- Native selects drop their arrow and use a 16px ink-soft chevron.

### Navigation
Sidebar on Sidebar Parchment with a right rule. Items carry an icon; the active row gets the sidebar-active fill, semibold ink text and a 3px inset bar in its group's tab colour. Group labels are tracked uppercase, tinted toward the tab colour.

### Stamp (signature)
Status labels: Geist Mono bold 11px uppercase, tracked, currentColor border with a hairline outer outline, tilted -2 degrees. Tone maps to colour: good green, wait amber, bad red, info navy, quiet ink-soft. Used for email, domain, contact, import and campaign status.

### Sending-health meters
A 6px rounded track at 10% ink with a tone-coloured fill, paired with a mono "rate / limit" readout. The sidebar strip is the compact version.

### Tables
Framed in a panel-wobble sketch; uppercase label headers at 44px; rows divided by dashed ink at 15%; hover fills Muted Paper at half strength.

## Do's and Don'ts

### Do:
- **Do** use Parchment for the page and Paper for every card, input and popover.
- **Do** use Carrier Navy (#1f3d7a) for the single primary action per view and for links.
- **Do** set every number in Geist Mono and every title in Source Serif 4.
- **Do** show status as a Stamp in the tone's colour, never as a pill.
- **Do** give sketch surfaces a 2px ink border with uneven corners, calmer on large panels.
- **Do** tint a sidebar group only with its own tab colour.

### Don't:
- **Don't** use flat white cards, pill badges or a neutral grey sidebar.
- **Don't** use green, amber or red for decoration; they only mean good, waiting, danger.
- **Don't** use blurred drop shadows; depth is the ink offset on sketch surfaces only.
- **Don't** spread tab colours beyond the sidebar.
- **Don't** copy another product's look; these values are this product's own.
- **Don't** ship dark mode surfaces as verified; the dark tokens have no toggle yet.
