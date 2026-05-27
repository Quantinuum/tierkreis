from tierkreis.builder import Graph
from tierkreis.controller.data.models import TKR
from tierkreis.builtins.stubs import iadd

sub_builder = Graph(inputs_type=TKR[int], outputs_type=TKR[int])
a = sub_builder.inputs
sub_workflow = sub_builder.finish_with_outputs(a)

builder = Graph(outputs_type=TKR[int])
a = builder.const(3)
b = builder.const(5)


c = builder.task(iadd(a=a, b=b))

d = builder.eval(sub_workflow, c)

workflow = builder.finish_with_outputs(d).data
