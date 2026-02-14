#!/usr/bin/env python3

"""Plot temperature log data into a SVG graph with Gnuplot"""

#
# Requirements :
# - Python >= 3.10
# - Gnuplot (gnuplot-nox is fine)
#

import argparse
import contextlib
import csv
import datetime
import gzip
import json
import subprocess
import tempfile


if __name__ == "__main__":
    arg_parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.ArgumentDefaultsHelpFormatter
    )
    arg_parser.add_argument(
        "log_files",
        nargs="+",
        help="Input log files (.jsonl or .jsonl.gz), in data chronological order",
    )
    arg_parser.add_argument("svg_filepath", help="Output SVG file")
    args = arg_parser.parse_args()

    devices = set()

    with tempfile.NamedTemporaryFile(
        "wt", suffix=".csv", delete_on_close=False
    ) as csv_file:
        with contextlib.closing(csv_file):
            csv_writer = csv.writer(csv_file)
            for log_filepath in args.log_files:
                with contextlib.ExitStack() as cm:
                    if log_filepath.endswith(".gz"):
                        jsonl_file = cm.enter_context(gzip.open(log_filepath, "rt"))
                    else:
                        jsonl_file = cm.enter_context(open(log_filepath))
                    for jsonl_line in map(json.loads, jsonl_file):
                        dt = datetime.datetime.fromisoformat(jsonl_line["time_utc"])
                        temps = {
                            measure["device"]: measure["temp_celcius"]
                            for measure in jsonl_line["measures"]
                        }
                        for device in temps.keys():
                            devices.add(device)
                        row: list[float | None] = [dt.timestamp()]
                        row.extend(temps.get(d) for d in devices)
                        csv_writer.writerow(row)

        with tempfile.TemporaryFile("w+t") as gnuplot_script_file:
            plots = ", ".join(
                f"'{csv_file.name}' using 1:{i + 2} with lines title '{d}'"
                for i, d in enumerate(devices)
            )
            gnuplot_lines = [
                "set terminal svg size 1449,900",
                f"set output {args.svg_filepath!r}",
                "set datafile separator comma",
                "set timefmt '%s'",
                "set xdata time",
                "set format x '%H:%M'",
                "set xlabel 'Time'",
                "set ylabel 'Temperature (°C)'",
                f"plot {plots}",
            ]
            gnuplot_script_file.write(";\n".join(gnuplot_lines))
            gnuplot_script_file.seek(0)

            subprocess.run(
                ["gnuplot"], text=True, stdin=gnuplot_script_file.fileno(), check=True
            )
