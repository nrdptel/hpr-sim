# PerfectFlite `.pf2` flight logs

A `.pf2` file is the flight log a PerfectFlite altimeter's software saves: the Pnut, the
StratoLogger and the StratoLoggerCF. It is plain text: a few lines about the altimeter and the
flight, then one row per sample, about 20 a second, with the time, altitude, speed, temperature
and battery voltage. A PerfectFlite has a barometer and no accelerometer, so every height and
speed in it comes from air pressure.

**To read a flight from one**, run [`hpr analyze`](../cli.md#hpr-analyze) on it, or see
[Flight-log readings](../physics/log-readings.md) for what is read and how.

This page is the reference for hpr's reader. PerfectFlite publishes no specification of the
format, so everything here was learned from exported files, by Debrief, the project owner's
earlier flight-log analyzer, whose reader this one follows. One real file has been read: Debrief's
public Pnut log. **How far to trust it:** the layout below is what that file and Debrief's
reader agree on. A variant it doesn't cover is refused or noted, not guessed at.

Code: `hpr_flightdata::perfectflite`
([API reference](../api/hpr_flightdata/perfectflite/index.html)), written for
[M4.2d](../decisions-and-roadmap.md#m4-2d).

## The layout

```text
PerfectFlite Pnut
Firmware: 1.0
Software: 1.1
Serial Number: 0
Apogee: 1281' AGL
Ground Elevation: 600' MSL
NumSamps: 984
Flight Number: 1
Comments: invented for hpr-sim's tests; no real flight's data

Data: (Time, Altitude, Velocity, Temperature (F), Voltage)
0.00, 0, 0, 70.00, 4.20
0.05, 0, 0, 70.00, 4.20
0.10, 0, 0, 70.00, 4.20
0.15, 0, 0
```

That is the head of the [invented log](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/logs/synthetic-pnut.pf2)
the tests and the [command-line guide](../cli.md#hpr-analyze) read.

| part | what hpr reads |
|---|---|
| first line | the altimeter's name. It must contain `PerfectFlite`, or the file is refused |
| `Apogee:` | the apogee the altimeter worked out, in feet above the pad, marked `'`. A value such as `PWRLOSS` (the power failed in flight) is noted, not read |
| `Ground Elevation:` | the pad's height above sea level, in feet, marked `'` |
| `NumSamps:` | the sample count; if the rows differ, a note says so |
| `Serial Number:`, `Firmware:`, `Flight Number:` | kept as the file states them |
| `Data:` | the columns, in order |
| the rows | numbers separated by commas, one row per sample |

Other `Key: value` lines, such as `Software:` and `Comments:`, are skipped.

The columns and their units:

| column | unit in the file | hpr's unit |
|---|---|---|
| `Time` | seconds from the altimeter's start | s |
| `Altitude` | feet above the altimeter's reading on the pad | m |
| `Velocity` | feet per second, up; the altimeter works it out from its own altitude | m/s |
| `Temperature (F)` | degrees Fahrenheit, inside the electronics bay | K |
| `Voltage` | volts, the battery | V |

A row may stop after the speed: the temperature and voltage are logged less often. A missing
cell is a gap, stored as `NaN`. A foot is 0.3048 m exactly.

## What is refused, and what is noted

A file is refused, with its line number, when:

- a row holds something that isn't a finite number, or more values than there are columns;
- a time doesn't come after the one before it;
- a row stops before its time or its altitude;
- the `Data:` line names no time or no altitude column, or names one twice;
- a stated height isn't marked as feet: hpr knows only feet, and won't read metres as feet;
- a line after the rows began isn't a row.

These are noted and read around:

- no `Data:` line: the columns are taken to be the five above, in that order, as Debrief takes them;
- a column the reader doesn't know: left out;
- a line in the header that isn't `Key: value`: skipped;
- a sample count that differs from `NumSamps:`;
- a stated apogee that isn't a height.

## Where this comes from

Debrief's `lib/parsers/perfectflite.ts` (MIT, the project owner's own) reads the same layout. It
was written from exported files and cites no document. hpr's reader differs in one way: Debrief
assumes the column order, and hpr takes it from the `Data:` line when there is one. Debrief's
public Pnut log states its apogee with a foot mark, `1009' AGL`, and its rows agree with that
figure ([Flight-log readings](../physics/log-readings.md#checked-against)).
