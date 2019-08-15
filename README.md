# rankonvert

Converts Auros sauce map rating JSONs to pandas readable CSVs

## Usage

`./rankonvert.exe input.json output.csv 4` where `input.json` is an existing input file, `output.csv` is a new output file and `4` is the max number of concurrent downloads.

If the process stales, you can `Ctrl-C` out of it, the currently parsed data will still be written.

## Resources

The input file currently used by Beat Boards is available at `resources/input.json`.