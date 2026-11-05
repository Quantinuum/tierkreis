cwlVersion: v1.2
class: CommandLineTool
id: concatenate
baseCommand: cat
inputs:
  source:
    type: File
    inputBinding:
      position: 2
  options:
    type: string[]
    inputBinding:
      position: 1
stdout: output.txt
outputs:
  result:
    type: File
    outputBinding:
      glob: output.txt
