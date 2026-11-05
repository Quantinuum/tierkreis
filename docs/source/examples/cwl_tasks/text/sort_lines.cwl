cwlVersion: v1.2
class: CommandLineTool
id: sort_lines
baseCommand: sort
inputs:
  source:
    type: File
    inputBinding:
      position: 1
stdout: sorted.txt
outputs:
  result:
    type: File
    outputBinding:
      glob: sorted.txt
