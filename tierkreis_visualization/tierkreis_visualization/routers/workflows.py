import json
import logging
from pathlib import Path
from typing import Annotated
from uuid import UUID

from fastapi import APIRouter, HTTPException, Query, status
from starlette.responses import JSONResponse, PlainTextResponse
from starlette.websockets import WebSocket, WebSocketDisconnect

from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.graph import NodeDef
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis_visualization.app_config import Request
from tierkreis_visualization.data.graph import get_node_data, parse_node_location
from watchfiles import awatch  # type: ignore

from tierkreis_visualization.data.eval import get_eval_node
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


@router.get("/{workflow_id}/graphs", response_model=GraphsResponse)
def list_nodes(
    request: Request, workflow_id: UUID, locs: Annotated[list[Loc], Query()]
) -> GraphsResponse:
    storage = request.app.state.get_storage_fn(workflow_id)
    print(locs)
    return GraphsResponse(graphs={loc: get_node_data(storage, loc) for loc in locs})


@router.get("/{workflow_id}/nodes/{node_location_str}")
def get_node(request: Request, workflow_id: UUID, node_location_str: str) -> PyGraph:
    node_location = parse_node_location(node_location_str)
    storage = request.app.state.get_storage_fn(workflow_id)
    return get_node_data(storage, node_location)


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
    node_location = parse_node_location(node_location_str)
    storage = request.app.state.get_storage_fn(workflow_id)
    bs = storage.read_output(node_location, PortID(port_name))
    try:
        return JSONResponse(json.loads(bs))
    except FileNotFoundError as e:
        return PlainTextResponse(str(e))


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
