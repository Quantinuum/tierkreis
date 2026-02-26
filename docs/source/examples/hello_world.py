from example_workers.hello_world_worker.api.stubs import greet

from tierkreis.builder import GraphBuilder
from tierkreis.controller.data.models import TKR

from hello_world_worker import greet


graph = GraphBuilder(inputs_type=TKR[str], outputs_type=TKR[str])
hello = graph.const("Hello ")
output = graph.task(greet(greeting=hello, subject=graph.inputs))
graph.outputs(output)
