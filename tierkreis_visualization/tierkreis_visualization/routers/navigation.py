from uuid import UUID

from tierkreis_core import Loc


def breadcrumb_links(crumbs: list[str]) -> list[tuple[str, str]]:
    url_path = ""
    links: list[tuple[str, str]] = []
    for crumb in crumbs:
        url_path = url_path + crumb
        links.append((crumb, url_path))

    return links


def breadcrumbs(workflow_id: UUID, node_location: Loc) -> list[tuple[str, str]]:
    node_location_strs: list[str] = [str(x) for x in node_location.steps()]
    static_links = ["/workflows", f"/{workflow_id}/nodes/-"]
    return breadcrumb_links(static_links + node_location_strs[1:])
