#!/usr/bin/env python3
"""Plot temperature log data into a SVG graph with Gnuplot"""

import csv
import contextlib
import datetime
import json
import subprocess
import sys
import tempfile

if __name__ == "__main__":
    jsonl_filepath = sys.argv[1]
    svg_filepath = sys.argv[2]

    devices = set()

    with tempfile.NamedTemporaryFile(
        "wt", suffix=".csv", delete_on_close=False
    ) as csv_file:
        with contextlib.closing(csv_file), open(jsonl_filepath) as jsonl_file:
            csv_writer = csv.writer(csv_file)
            for jsonl_line in map(json.loads, jsonl_file):
                dt = datetime.datetime.fromisoformat(jsonl_line["time_utc"])
                temps = {
                    measure["device"]: measure["temp_celcius"]
                    for measure in jsonl_line["measures"]
                }
                for device in temps.keys():
                    devices.add(device)
                row = [dt.timestamp()]
                row.extend(
                    str(temps.get(d, "")) if temps.get(d) is not None else ""
                    for d in devices
                )
                csv_writer.writerow(row)

        with tempfile.TemporaryFile("w+t") as gnuplot_script_file:
            plots = ", ".join(
                f"'{csv_file.name}' using 1:{i + 2} with lines title '{d}'"
                for i, d in enumerate(devices)
            )
            gnuplot_lines = [
                "set terminal svg size 1449,900",
                f"set output {svg_filepath!r}",
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
