import json
import logging
from pathlib import Path
from typing import Annotated
from uuid import UUID

from fastapi import APIRouter, HTTPException, Query, status
from starlette.responses import JSONResponse, PlainTextResponse
from starlette.websockets import WebSocket, WebSocketDisconnect

from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.location import Loc, WorkerCallArgs
from tierkreis.controller.storage.base import ControllerStorage
from tierkreis.exceptions import TierkreisError
from watchfiles import awatch  # type: ignore

from tierkreis_visualization.data.workflows import WorkflowDisplay, get_workflows
from tierkreis_visualization.routers.models import GraphsResponse, PyGraph

router = APIRouter()
logger = logging.getLogger(__name__)


@router.websocket("/{workflow_id}/nodes/{node_location_str}")
async def websocket_endpoint(
    websocket: WebSocket, workflow_id: UUID, node_location_str: str
) -> None:
    if workflow_id.int == 0:
        return
    storage = websocket.app.state.get_storage_fn(workflow_id)
    try:
        await websocket.accept()
        await handle_websocket(websocket, node_location_str, storage)
    except WebSocketDisconnect:
        pass


async def handle_websocket(
    websocket: WebSocket,
    node_location_str: str,
    storage: ControllerStorage,
) -> None:
    node_location = parse_node_location(node_location_str)

    async for changes in awatch(storage.workflow_dir, recursive=True):
        relevant_changes: set[str] = set()
        for change in changes:
            path = Path(change[1]).relative_to(storage.workflow_dir)
            if not path.parts:
                continue
            loc = path.parts[0]
            if loc.startswith(node_location):
                relevant_changes.add(loc)

        if relevant_changes:
            await websocket.send_json(list(relevant_changes))


@router.get("/")
def list_workflows(request: Request) -> list[WorkflowDisplay]:
    try:
        storage_type = request.app.state.storage_type
        return get_workflows(storage_type)
    except FileNotFoundError:
        raise HTTPException(
            status.HTTP_404_NOT_FOUND,
            "Workflow not found, make sure the workflow exists in the workflow directory.",
        )


class NodeResponse(BaseModel):
    definition: WorkerCallArgs


def parse_node_location(node_location_str: str) -> Loc:
    return Loc(node_location_str)


def get_errored_nodes(storage: ControllerStorage) -> list[Loc]:
    errored_nodes = storage.read_errors(Loc("-"))
    return [parse_node_location(node) for node in errored_nodes.split("\n")]


def get_node_data(
    workflow_id: UUID, loc: Loc, storage: ControllerStorage
) -> dict[str, Any]:
    errored_nodes = get_errored_nodes(storage)

    try:
        node = storage.read_node_def(loc)
    except FileNotFoundError:
        return {
            "breadcrumbs": breadcrumbs(workflow_id, loc),
            "url": f"/workflows/{workflow_id}/nodes/{loc}",
            "node_location": str(loc),
            "name": "unavailable.jinja",
        }

    ctx: dict[str, Any] = {}
    match node.type:
        case "eval":
            data = get_eval_node(storage, loc, errored_nodes)
            name = "eval.jinja"
            ctx = PyGraph(nodes=data.nodes, edges=data.edges).model_dump()

        case "loop":
            data = get_loop_node(storage, loc, errored_nodes)
            name = "loop.jinja"
            ctx = PyGraph(nodes=data.nodes, edges=data.edges).model_dump(
                by_alias=True, mode="json"
            )
        case "map":
            data = get_map_node(storage, loc, node, errored_nodes)
            name = "map.jinja"
            ctx = PyGraph(nodes=data.nodes, edges=data.edges).model_dump(
                by_alias=True, mode="json"
            )

        case "function":
            try:
                definition = storage.read_worker_call_args(loc)
            except FileNotFoundError:
                return {
                    "breadcrumbs": breadcrumbs(workflow_id, loc),
                    "url": f"/workflows/{workflow_id}/nodes/{loc}",
                    "node_location": str(loc),
                    "name": "unavailable.jinja",
                }
            data = get_function_node(storage, loc)
            name = "function.jinja"
            ctx = {
                "definition": definition.model_dump(mode="json"),
                "data": data.model_dump(mode="json"),
            }
        case "const" | "ifelse" | "eifelse" | "input" | "output":
            name = "fallback.jinja"
            parent = loc.parent()
            if parent is None:
                raise TierkreisError("Visualisable node should have parent.")

            inputs = {k: (parent.N(i), p) for k, (i, p) in node.inputs.items()}
            outputs = {k: (loc, k) for k in node.outputs}
            ctx = {"node": node, "inputs": inputs, "outputs": outputs}

        case _:
            assert_never(node)

    ctx["breadcrumbs"] = breadcrumbs(workflow_id, loc)
    ctx["url"] = f"/workflows/{workflow_id}/nodes/{loc}"
    ctx["node_location"] = str(loc)
    ctx["name"] = name

    return ctx


async def node_stream(
    workflow_id: UUID, node_location: Loc, storage: ControllerStorage
):
    node_path = CONFIG.tierkreis_path / str(workflow_id) / str(node_location)
    async for _changes in awatch(node_path, recursive=False):
        if (node_path / "definition").exists():
            ctx = get_node_data(workflow_id, node_location, storage)
            yield f"event: message\ndata: {json.dumps(ctx)}\n\n"


@router.get("/{workflow_id}/nodes/{node_location_str}")
def get_node(request: Request, workflow_id: UUID, node_location_str: str) -> PyGraph:
    node_location = parse_node_location(node_location_str)
    storage = request.app.state.get_storage_fn(workflow_id)
    return get_node_data(storage, node_location)


@router.get("/{workflow_id}/nodes/{node_location_str}/outputs")
def get_eval_outputs(
    request: Request,
    workflow_id: UUID,
    node_location_str: str,
):
    loc = parse_node_location(node_location_str)
    storage = request.app.state.get_storage_fn(workflow_id)
    outputs = storage.read_output_ports(loc)
    out = {k: str(storage.read_output(loc, k)) for k in outputs}
    return JSONResponse(out)


@router.get("/{workflow_id}/nodes/{node_location_str}/inputs/{port_name}")
def get_input(
    request: Request,
    workflow_id: UUID,
    node_location_str: str,
    port_name: str,
):
    try:
        node_location = parse_node_location(node_location_str)
        storage = request.app.state.get_storage_fn(workflow_id)
        definition = storage.read_worker_call_args(node_location)

        with open(definition.inputs[port_name], "rb") as fh:
            return JSONResponse(json.loads(fh.read()))
    except FileNotFoundError as e:
        return PlainTextResponse(str(e))


@router.get("/{workflow_id}/nodes/{node_location_str}/outputs/{port_name}")
def get_output(
    request: Request,
    workflow_id: UUID,
    node_location_str: str,
    port_name: str,
):
    loc = parse_node_location(node_location_str)
    storage = request.app.state.get_storage_fn(workflow_id)
    return PlainTextResponse(outputs_from_loc(storage, loc, port_name))


@router.get("/{workflow_id}/logs")
def get_logs(
    request: Request,
    workflow_id: UUID,
) -> PlainTextResponse:
    storage = request.app.state.get_storage_fn(workflow_id)
    logs = storage.read(storage.logs_path)
    return PlainTextResponse(logs)


@router.get("/{workflow_id}/nodes/{node_location_str}/errors")
def get_errors(
    request: Request,
    workflow_id: UUID,
    node_location_str: str,
):
    node_location = parse_node_location(node_location_str)
    storage = request.app.state.get_storage_fn(workflow_id)
    if not storage.node_has_error(node_location):
        return PlainTextResponse("Node has no errors.", status_code=404)

    messages = storage.read_errors(node_location)
    return PlainTextResponse(messages)
