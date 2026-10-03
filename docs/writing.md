# Writing these pages

**This page is the house style for hpr-sim's documentation.** It is for anyone who writes or
edits a page of this site, a model page, or a doc comment. It applies to every new page and to
every page a change touches. Older pages move to it as they are edited, not all at once.

The aim is simple: a hobby rocketeer who knows some physics and some code, but not this project,
reads a page once and knows what it says and how far to trust it.

## The rules

1. **Bottom line first.** The first sentence of a page, a section or a paragraph says what it
   concludes. The detail and the reasoning come after.
2. **One idea a sentence.** Aim for about 20 words a sentence on average. A sentence over 25
   words is flagged for a second look; it is not forbidden.
3. **Short paragraphs.** Five sentences at most.
4. **Numbers carry their meaning.** Every number has a unit, and says what it is compared with:
   "apogee 1.2% above RocketPy's", not "1.2% error".
5. **One fact, one precision.** The same fact is written with the same precision everywhere it
   appears. If one page gives the real-flight apogee error as 6.04%, no other page rounds it to 6%.
6. **Labels are links.** An internal label, such as a milestone or a decision record, is a link
   with a few words of meaning, never bare. The site's checks enforce this one
   ([the documentation site decision record](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-016-the-documentation-site-mdbook-over-docs-and-checks-for-links-labels-and-equations-2026-09-18)).
7. **Words before equations.** Say in words what an equation does, give the equation, then work
   an example with real numbers.

## Four kinds of page

Pages follow Diátaxis, a common way to sort documentation by what the reader is doing. Each page
is one kind; a page that mixes them is split when it is next touched.

| kind | the reader wants to | example on this site |
|---|---|---|
| Tutorial | learn by doing, start to finish | [Getting started](getting-started.md) |
| How-to guide | get one task done | [Launch-day weather](weather.md) |
| Reference | look a fact up | [The command line](cli.md) |
| Explanation | understand why | [How a flight is simulated](how-a-flight-is-simulated.md) |

## Measured, not gated

Readability is measured, not used to fail a build. A planned `cargo xtask` report will print, for
each page, the average sentence length, the share of sentences over 25 words, and how many
internal labels it holds per 100 words. The report shows where to look; a person decides what to
change.

There is one hard check, planned with the bookkeeping clean-up
([M0.5 milestone](decisions-and-roadmap.md#m0-5)): a model page's *In short* box holds at most
about 150 words and five bullets.

The targets for user pages, measured at the next review of the project: at most 20 words a
sentence on average, and fewer than 10% of sentences over 30 words.

## Where this comes from

The rules follow the UK Government Digital Service's style guide, and the sentence-length checks
that Datadog and GitLab run with the Vale linter. The page kinds are Daniele Procida's Diátaxis
framework. The decision to adopt them is in the
[check-in decision record](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-144-the-2026-10-03-check-in-leaner-bookkeeping-issues-writing-guardrails-and-licensing-records-2026-10-03).
