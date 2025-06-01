# OSM road bundler

This is an **experimental** tool for simplifying OSM data. It aims to:

- collapse roundabouts to a single point
- simplify dog-leg intersections into simple 4-way intersections
- zip together both sides of a dual carriageway into one road
- match sidewalks and cycleways tagged separately to their road, and
  distinguish proper off-road cases

All of these simplifications are done **losslessly** -- the data from the
original OSM ways is bundled onto the final simplified road.

<!-- TODO: Example picture -->
A situation like https://www.openstreetmap.org/#map=18/52.478428/13.446053 with
a dual carriageway and parallel sidewalks would transform the 4 parallel lines
into one synthetic road, preserving links to the original OSM ways with full
details and providing a consolidated "cross-section view" of the road from
left-to-right.

Try the **experimental prototype** at https://dabreegster.github.io/road-bundler.

## Status

This is an early experiment. Don't depend on it yet. Please coordinate ideas /
use cases through GH issues.

Follow along with the [dev
log](https://github.com/dabreegster/road-bundler/issues/1) for updates.

## Related work

See https://github.com/a-b-street/osm2streets/discussions/195 for lots of
context and similar projects. [neatnet](https://uscuni.org/neatnet) is one
newer, promising project.

Unlike other projects, this one:

- pays attention to sidewalks and cycleways, not just the driving network
- preserves information about the original OSM edges that get transformed,
  instead of just producing a simplified geometry
- makes it really convenient to try anywhere, from a web app with no
  installation, and to interactively debug what the operations do and which are
  appropriate

## Use cases

The network transformations that should be run and the level of detail
preserved really does depend on the particular use case, so this project aims
for a level of flexibility. Ultimately, the way downstream projects could use
this one is probably through a two-part process:

1.  Use the interactive web app to figure out which transformations make sense
    in a given area for some use case
2.  Apply to osm.xml or osm.pbf data through a CLI tool, a JS library, or other
    bindings and get a simplified GeoJSON network as output, with structured
    properties as output

Some example use cases from my other projects:

- Creating a simplified graph for od2net and NPW. Directionality on dual
  carriageways doesn't matter for those, and network planning in complex areas
  is unnecessarily clunky right now.
- Detecting
  [is_sidepath](https://wiki.openstreetmap.org/wiki/Proposal:Key:is_sidepath)
  automatically, with uses like answering "what's the speed on the road that a
  sidewalk/cycleway runs next to?" or producing routing directions like "walk
  along the sidewalk of Main Street, then cross over".
- Revisiting how osm2streets renders complex streets in detail
- Letting a Streetmix-like cross-section editor operate on a complex street as
  one logical unit
- Calculating a level of traffic stress for one simplified entity in
  [CQI](https://www.osm-verkehrswende.org/cqi/map/?anzeige=cqi&map=18.2%2F13.45162%2F52.47809&filters=usable-yes)
