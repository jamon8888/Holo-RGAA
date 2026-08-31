# criteria — RGAA 4.1.2 Knowledge Base

## Overview

Complete reference for all 106 RGAA 4.1.2 criteria organized by topic. Each criterion includes:
- Criterion ID and title
- Classification (Deterministe, IaAssiste, Manuel)
- What it tests
- How to verify (automated, AI-assisted, or manual)
- WCAG 2.2 and EN 301 549 cross-references
- Common failure patterns and fixes

## Topic 1: Images (Critères 1.1–1.9)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 1.1 | Each image has an alternative | Deterministe | axe-core: image-alt |
| 1.2 | Decorative images are ignored | Deterministe | axe-core: image-alt |
| 1.3 | Complex images have a detailed description | IaAssiste | Holo3 visual evaluation |
| 1.4 | Images of text are avoided | Deterministe | Manual check |
| 1.5 | When an image cannot be displayed, alternative is provided | IaAssiste | Holo3 evaluation |
| 1.6 | Image legends are grouped with their image | Deterministe | Manual check |
| 1.7 | Images of text in an SVG have an alternative | Deterministe | axe-core check |
| 1.8 | Animated images are avoided or provide controls | IaAssiste | Holo3 evaluation |
| 1.9 | Color is not the only means of conveying information | Deterministe | axe-core: color-contrast |

**WCAG refs:** 1.1.1, 1.2.1, 1.4.3, 1.4.11

## Topic 2: Colors (Critères 2.1–2.5)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 2.1 | Foreground and background colors can be overridden | Deterministe | User stylesheet check |
| 2.2 | Color contrast is sufficient | Deterministe | axe-core: color-contrast |
| 2.3 | Moving or blinking content can be paused | IaAssiste | Holo3 evaluation |
| 2.4 | Color is not the only means of conveying information | Deterministe | Same as 1.9 |
| 2.5 | Text can be resized up to 200% | IaAssiste | Holo3 evaluation |

**WCAG refs:** 1.4.3, 2.2.1, 2.2.2, 2.3.1

## Topic 3: Content Structure (Critères 3.1–3.6)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 3.1 | Information and relationships are conveyed through structure | Deterministe | axe-core: region |
| 3.2 | All content in the DOM is in a meaningful order | Deterministe | DOM order check |
| 3.3 | Language is specified in the HTML element | Deterministe | axe-core: html-has-lang |
| 3.4 | Language changes are identified | Deterministe | axe-core: lang-accurate |
| 3.5 | Headings describe the topic or purpose | Deterministe | axe-core: heading-order |
| 3.6 | Quote marks are correctly used | Manuel | Manual check |

**WCAG refs:** 1.3.1, 3.1.1, 3.1.2

## Topic 4: Tables (Critères 4.1–4.4)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 4.1 | Data tables have headers | Deterministe | axe-core: th-has-data-cells |
| 4.2 | Data tables have a caption | Deterministe | axe-core: table-complex |
| 4.3 | Complex data tables are simplified | IaAssiste | Holo3 evaluation |
| 4.4 | Layout tables do not use data table markup | Deterministe | Manual check |

**WCAG refs:** 1.3.1, 2.4.6

## Topic 5: Links (Critères 5.1–5.4)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 5.1 | Each link's purpose is clear from its text | Deterministe | axe-core: link-in-text-block |
| 5.2 | Each link's purpose is clear from context | IaAssiste | Holo3 evaluation |
| 5.3 | Links with same label have same destination | Deterministe | axe-core: identical-links |
| 5.4 | Link text is not empty or "click here" | Deterministe | Manual check |

**WCAG refs:** 2.4.4, 2.4.9

## Topic 6: Scripts (Critères 6.1–6.6)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 6.1 | Information is not conveyed only through scripts | Deterministe | Fallback content check |
| 6.2 | User interface components are keyboard operable | Deterministe | axe-core: region |
| 6.3 | Event handlers are keyboard operable | Deterministe | Manual check |
| 6.4 | Focus order does not trap keyboard users | Manuel | Guided test |
| 6.5 | Focus is visible on interactive elements | Manuel | Guided test |
| 6.6 | Status messages are programmatically determined | Deterministe | axe-core: aria-alert-status |

**WCAG refs:** 2.1.1, 2.4.3, 2.4.7, 4.1.3

## Topic 7: Mandatory Elements (Critères 7.1–7.5)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 7.1 | HTML is valid | Deterministe | html validate |
| 7.2 | CSS is valid | Deterministe | css validate |
| 7.3 | Language used is declared | Deterministe | Same as 3.3 |
| 7.4 | Parsing is consistent across browsers | Deterministe | Browser rendering check |
| 7.5 | Pages work without CSS/JavaScript | Deterministe | Disable CSS/JS check |

**WCAG refs:** 4.1.1, 4.1.2

## Topic 8: Presentation (Critères 8.1–8.10)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 8.1 | No duplicate content without reference | Deterministe | Manual check |
| 8.2 | Page has a language | Deterministe | Same as 3.3 |
| 8.3 | Blinking content can be stopped | Deterministe | axe-core: blink |
| 8.4 | Content renders correctly at 320px width | IaAssiste | Holo3 evaluation |
| 8.5 | Text spacing can be overridden | Deterministe | Stylesheet check |
| 8.6 | Images of text are avoided | Deterministe | Same as 1.4 |
| 8.7 | Interactive elements have visible labels | Deterministe | axe-core |
| 8.8 | Content does not rely solely on hover/focus | IaAssiste | Holo3 evaluation |
| 8.9 | Touch targets are large enough | Deterministe | axe-core: target-size |
| 8.10 | Zoom is not disabled | Deterministe | viewport meta check |

**WCAG refs:** 1.4.4, 1.4.10, 1.4.11, 2.5.5

## Topic 9: Forms (Critères 9.1–9.11)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 9.1 | Form fields have visible labels | Deterministe | axe-core: label |
| 9.2 | Related form fields are grouped | Deterministe | axe-core: fieldset |
| 9.3 | Labels are positioned correctly | Deterministe | axe-core |
| 9.4 | Input type is appropriate | Deterministe | input type check |
| 9.5 | Autocomplete attributes are used | IaAssiste | Manual check |
| 9.6 | Required fields are indicated | Deterministe | axe-core: required-attr |
| 9.7 | Error messages are helpful | IaAssiste | Holo3 evaluation |
| 9.8 | Error prevention for legal/data entry | Deterministe | axe-core |
| 9.9 | Help is available for form fields | IaAssiste | Holo3 evaluation |
| 9.10 | Labels describe the input purpose | Deterministe | axe-core |
| 9.11 | CAPTCHA alternatives are provided | Manuel | Manual check |

**WCAG refs:** 1.3.1, 2.4.6, 3.3.1, 3.3.2

## Topic 10: Navigation (Critères 10.1–10.13)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 10.1 | Skip links are provided | Deterministe | axe-core: bypass |
| 10.2 | Navigation is consistent across pages | IaAssiste | Holo3 evaluation |
| 10.3 | Navigation can be bypassed | Deterministe | axe-core: bypass |
| 10.4 | Headings and labels describe the topic | Deterministe | Same as 3.5 |
| 10.5 | Current page is indicated in navigation | Deterministe | aria-current check |
| 10.6 | List structure is used for related items | Deterministe | axe-core: list |
| 10.7 | Focus management is correct | Manuel | Guided test |
| 10.8 | Tab order is logical | Deterministe | DOM order check |
| 10.9 | Search is available if site has search | IaAssiste | Holo3 evaluation |
| 10.10 | Sections have headings | Deterministe | axe-core: heading-order |
| 10.11 | Navigation has no broken links | Deterministe | Link validation |
| 10.12 | Breadcrumbs are provided | IaAssiste | Holo3 evaluation |
| 10.13 | Items in a menu are clearly separated | Deterministe | Visual check |

**WCAG refs:** 2.4.1, 2.4.3, 2.4.6, 2.4.9

## Topic 11: Accessibility Declaration (Critères 11.1–11.3)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 11.1 | Accessibility declaration is present | Manuel | Manual check |
| 11.2 | Declaration is up to date | Manuel | Manual check |
| 11.3 | Feedback channels are provided | Manuel | Manual check |

## Topic 12: Assistive Technologies (Critères 12.1–12.5)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 12.1 | ARIA roles are valid | Deterministe | axe-core: aria-valid-attr |
| 12.2 | ARIA properties are valid | Deterministe | axe-core: aria-valid-attr-value |
| 12.3 | ARIA is used correctly | IaAssiste | Holo3 evaluation |
| 12.4 | Dynamic content is announced | Deterministe | axe-core: aria-alert-status |
| 12.5 | No ARIA if native HTML equivalent | IaAssiste | Holo3 evaluation |

**WCAG refs:** 4.1.2, 4.1.3

## Topic 13: Video/Audio (Critères 13.1–13.13)

| ID | Title | Class | Test Method |
|----|-------|-------|------------|
| 13.1 | Alternatives for prerecorded audio are provided | Deterministe | Transcript check |
| 13.2 | Captions are provided for prerecorded video | Deterministe | Caption file check |
| 13.3 | Audio description is provided for prerecorded video | IaAssiste | Holo3 evaluation |
| 13.4 | Alternatives for live audio are provided | Deterministe | Live caption check |
| 13.5 | Media player controls are accessible | Manuel | Guided test |
| 13.6 | Sign language interpretation is provided | IaAssiste | Holo3 evaluation |
| 13.7 | Transcript is provided for audio-only content | Deterministe | Transcript check |
| 13.8 | Controls for embedded media are keyboard accessible | Manuel | Guided test |
| 13.9 | No content flashes more than 3 times/second | Deterministe | axe-core: blink |
| 13.10 | Audio does not play automatically | Deterministe | autoplay check |
| 13.11 | Content can be paused or stopped | Deterministe | Controls check |
| 13.12 | Volume can be controlled independently | Deterministe | Volume control check |
| 13.13 | Audio description or alternative is provided | Deterministe | Same as 13.3 |

**WCAG refs:** 1.2.1–1.2.5, 2.2.2, 2.3.1

## Criterion ID Format

RGAA criterion IDs use dotted notation:
- `1.1` — Topic 1 (Images), Criterion 1
- `8.2` — Topic 8 (Presentation), Criterion 2
- `13.7` — Topic 13 (Video/Audio), Criterion 7

## Compliance Classification Summary

| Classification | Count | Description |
|---------------|-------|-------------|
| **Deterministe** | 77 | Fully automated testing with axe-core + gap-fix rules |
| **IaAssiste** | 22+ | AI-assisted evaluation with Holo3 LLM |
| **Manuel** | 7+ | Manual testing protocol required |

Total: **106 criteria** across **13 topics**
