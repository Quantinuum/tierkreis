export interface paths {
    "/api/info": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_info"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/workflows/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all workflows in the database, returning a summary of each.
         * @description # Errors
         *
         *     Returns an internal server error if the database query fails.
         */
        get: operations["list_workflows"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/workflows/{workflow_id}/graphs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get the graphs for a specific workflow.
         * @description # Errors
         *
         *     Returns an internal server error if the workflow is not found or if the graph cannot be built.
         */
        get: operations["list_nodes"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/workflows/{workflow_id}/logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get the logs for a specific workflow run, returning the log detail as a string.
         * @description # Errors
         *
         *     Returns an internal server error if the workflow run state cannot be loaded or if the logs cannot be loaded.
         */
        get: operations["get_workflow_logs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/workflows/{workflow_id}/nodes/{node_location_str}/errors": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get the error logs for a specific node in a workflow run, returning the error detail as a string.
         * @description # Errors
         *
         *     Returns an internal server error if the workflow run state cannot be loaded or if the error logs cannot be loaded.
         */
        get: operations["get_node_errors"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/workflows/{workflow_id}/nodes/{node_location_str}/inputs/{port_name}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get the input for a specific port of a node in a workflow run, returning the raw value as JSON or text.
         * @description # Errors
         *
         *     Returns an internal server error if the workflow run state cannot be loaded, if the node state cannot be read, or if the input value cannot be loaded.
         */
        get: operations["get_input"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/workflows/{workflow_id}/nodes/{node_location_str}/logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get the logs for a specific node in a workflow run, returning the log detail as a string.
         * @description # Errors
         *
         *     Returns an internal server error if the workflow run state cannot be loaded or if the logs cannot be loaded.
         */
        get: operations["get_node_logs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/workflows/{workflow_id}/nodes/{node_location_str}/outputs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all outputs for a specific node in a workflow run, returning a map of port name to value.
         * @description # Errors
         *
         *     Returns an internal server error if the workflow run state cannot be loaded, if the node state cannot be read, or if the outputs cannot be loaded.
         */
        get: operations["get_all_outputs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/workflows/{workflow_id}/nodes/{node_location_str}/outputs/{port_name}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List the output for a specific port of a node in a workflow run, returning the raw value as JSON or text.
         * @description # Errors
         *
         *     Returns an internal server error if the workflow run state cannot be loaded, if the node state cannot be read, or if the output value cannot be loaded.
         */
        get: operations["get_single_output"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/workflows/{workflow_id}/nodes/{node_location_str}/restart": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Restart a task node and return the locations invalidated by the restart.
         * @description # Errors
         *
         *     Returns an internal server error if the workflow run or node cannot be restarted.
         */
        post: operations["restart_node"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        /** @description Response containing graphs for multiple requested locations. */
        GraphsResponse: {
            /** @description Map from location string to the graph at that location. */
            graphs: {
                [key: string]: components["schemas"]["PyGraph"];
            };
        };
        /** @description Describes a connection into an input port of a node. */
        NodeInputs: {
            /** @description The location of the node providing the value. */
            from_node: string;
            /** @description The name of the output port on the source node. */
            from_port: string;
            /** @description The name of the input port on this node. */
            port: string;
        };
        /**
         * @description The status of a node in the workflow graph.
         *     TODO: Enable remaining states
         * @enum {string}
         */
        NodeStatus: "Not started" | "Started" | "Error" | "Finished";
        /** @description A directed edge in the workflow graph. */
        PyEdge: {
            /** @description Whether this edge is part of a conditional branch. */
            conditional: boolean;
            /** @description Location of the source node. */
            from_node: string;
            /** @description Name of the output port on the source node. */
            from_port: string;
            /** @description Location of the target node. */
            to_node: string;
            /** @description Name of the input port on the target node. */
            to_port: string;
            /** @description Loaded output value at this edge, if available and the node has completed. */
            value?: string | null;
        };
        /** @description The graph of nodes and edges at a particular location in the workflow. */
        PyGraph: {
            /** @description The edges connecting nodes in this graph. */
            edges: components["schemas"]["PyEdge"][];
            /** @description The nodes in this graph. */
            nodes: components["schemas"]["PyNode"][];
        };
        /** @description A node in the workflow graph, with its current execution status. */
        PyNode: {
            /** @description ISO-8601 timestamp when the node finished, or `""` if not finished. */
            finished_time: string;
            /** @description Human-readable display name derived from the node definition. */
            function_name: string;
            /** @description The location of this node as a string. */
            id: string;
            /** @description Incoming connections for each input port. */
            inputs: components["schemas"]["NodeInputs"][];
            /** @description Same as `id`; kept for Python API compatibility. */
            node_location: string;
            /** @description The structural type of the node. */
            node_type: string;
            /** @description Names of the output ports. */
            outputs: string[];
            /** @description ISO-8601 timestamp when the node started running, or `""` if not started. */
            started_time: string;
            /** @description Current execution status. */
            status: components["schemas"]["NodeStatus"];
            /** @description A human-readable value associated with the node (const value, input name, etc.). */
            value?: string | null;
        };
        /** @description Runtime metadata returned by `/api/info`. */
        RuntimeMetadata: {
            version: string;
        };
        /** @description Workflow display information returned by `/api/workflows/`. */
        WorkflowDisplay: {
            /** @description Errored Nodes are taken from Errored time */
            errors: string[];
            /**
             * Format: uuid
             * @description Currently the run UUID, no attempt/workflow
             */
            id: string;
            /** Format: int64 */
            id_int: number;
            name?: string | null;
            start_time: string;
            tkr_version: string;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    get_info: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RuntimeMetadata"];
                };
            };
        };
    };
    list_workflows: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkflowDisplay"][];
                };
            };
        };
    };
    list_nodes: {
        parameters: {
            query?: {
                /** @description Location strings to fetch graphs for (repeatable). */
                locs?: string[];
            };
            header?: never;
            path: {
                /** @description The workflow uuid */
                workflow_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GraphsResponse"];
                };
            };
            /** @description Error building graph */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_workflow_logs: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Run ID */
                workflow_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Log detail for the workflow */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Error loading log detail */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_node_errors: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Run ID */
                workflow_id: string;
                /** @description Location string */
                node_location_str: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Error detail for the node */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Error loading error detail */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_input: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Run ID */
                workflow_id: string;
                /** @description Location string */
                node_location_str: string;
                /** @description Output port name */
                port_name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Input value as JSON or text */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Input not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_node_logs: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Run ID */
                workflow_id: string;
                /** @description Location string */
                node_location_str: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Log detail for the node */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Error loading log detail */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_all_outputs: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Run ID */
                workflow_id: string;
                /** @description Location string */
                node_location_str: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description JSON object of port name to value */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Error loading outputs */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_single_output: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Run ID */
                workflow_id: string;
                /** @description Location string */
                node_location_str: string;
                /** @description Output port name */
                port_name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Raw output value as JSON or text */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Output not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    restart_node: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Run ID */
                workflow_id: string;
                /** @description Location string */
                node_location_str: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": string[];
                };
            };
            /** @description Error restarting node */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
}
