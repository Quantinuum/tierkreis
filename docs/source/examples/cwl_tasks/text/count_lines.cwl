cwlVersion: v1.2
class: CommandLineTool
id: count_lines
doc: Interface of the jq wrapper that converts wc -l output to a JSON integer.
inputs:
  source: File
outputs:
  count: int
