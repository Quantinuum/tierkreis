export interface paths {
    "/api/workflows/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List Workflows */
        get: operations["list_workflows_api_workflows__get"];
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
        /** List Nodes */
        get: operations["list_nodes_api_workflows__workflow_id__graphs_get"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/workflows/{workflow_id}/nodes/{node_location_str}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Node */
        get: operations["get_node_api_workflows__workflow_id__nodes__node_location_str__get"];
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
        /** Get Input */
        get: operations["get_input_api_workflows__workflow_id__nodes__node_location_str__inputs__port_name__get"];
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
        /** Get Output */
        get: operations["get_output_api_workflows__workflow_id__nodes__node_location_str__outputs__port_name__get"];
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
        /** Get Logs */
        get: operations["get_logs_api_workflows__workflow_id__logs_get"];
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
        /** Get Errors */
        get: operations["get_errors_api_workflows__workflow_id__nodes__node_location_str__errors_get"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/{path}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Read Root */
        get: operations["read_root__path__get"];
        put?: never;
        post?: never;
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
        /** Const */
        Const: {
            /** Value */
            value: unknown;
            /** Outputs */
            outputs?: string[];
            /** Inputs */
            inputs?: {
                [key: string]: [
                    number,
                    string
                ];
            };
            /**
             * Type
             * @default const
             * @constant
             */
            type: "const";
        };
        /** EagerIfElse */
        EagerIfElse: {
            /** Pred */
            pred: [
                number,
                string
            ];
            /** If True */
            if_true: [
                number,
                string
            ];
            /** If False */
            if_false: [
                number,
                string
            ];
            /** Outputs */
            outputs?: string[];
            /** Inputs */
            inputs?: {
                [key: string]: [
                    number,
                    string
                ];
            };
            /**
             * Type
             * @default eifelse
             * @constant
             */
            type: "eifelse";
        };
        /** Eval */
        Eval: {
            /** Graph */
            graph: [
                number,
                string
            ];
            /** Inputs */
            inputs: {
                [key: string]: [
                    number,
                    string
                ];
            };
            /** Outputs */
            outputs?: string[];
            /**
             * Type
             * @default eval
             * @constant
             */
            type: "eval";
        };
        /** Func */
        Func: {
            /** Function Name */
            function_name: string;
            /** Inputs */
            inputs: {
                [key: string]: [
                    number,
                    string
                ];
            };
            /** Outputs */
            outputs?: string[];
            /**
             * Type
             * @default function
             * @constant
             */
            type: "function";
        };
        /** GraphsResponse */
        GraphsResponse: {
            /** Graphs */
            graphs: {
                [key: string]: components["schemas"]["PyGraph"];
            };
        };
        /** HTTPValidationError */
        HTTPValidationError: {
            /** Detail */
            detail?: components["schemas"]["ValidationError"][];
        };
        /** IfElse */
        IfElse: {
            /** Pred */
            pred: [
                number,
                string
            ];
            /** If True */
            if_true: [
                number,
                string
            ];
            /** If False */
            if_false: [
                number,
                string
            ];
            /** Outputs */
            outputs?: string[];
            /** Inputs */
            inputs?: {
                [key: string]: [
                    number,
                    string
                ];
            };
            /**
             * Type
             * @default ifelse
             * @constant
             */
            type: "ifelse";
        };
        /** Input */
        Input: {
            /** Name */
            name: string;
            /** Outputs */
            outputs?: string[];
            /** Inputs */
            inputs?: {
                [key: string]: [
                    number,
                    string
                ];
            };
            /**
             * Type
             * @default input
             * @constant
             */
            type: "input";
        };
        /** Loop */
        Loop: {
            /** Body */
            body: [
                number,
                string
            ];
            /** Inputs */
            inputs: {
                [key: string]: [
                    number,
                    string
                ];
            };
            /** Continue Port */
            continue_port: string;
            /** Outputs */
            outputs?: string[];
            /**
             * Type
             * @default loop
             * @constant
             */
            type: "loop";
        };
        /** Map */
        Map: {
            /** Body */
            body: [
                number,
                string
            ];
            /** Inputs */
            inputs: {
                [key: string]: [
                    number,
                    string
                ];
            };
            /** Outputs */
            outputs?: string[];
            /**
             * Type
             * @default map
             * @constant
             */
            type: "map";
        };
        /** Output */
        Output: {
            /** Inputs */
            inputs: {
                [key: string]: [
                    number,
                    string
                ];
            };
            /** Outputs */
            outputs?: string[];
            /**
             * Type
             * @default output
             * @constant
             */
            type: "output";
        };
        /** PyEdge */
        PyEdge: {
            /** From Node */
            from_node: string;
            /** From Port */
            from_port: string;
            /** To Node */
            to_node: string;
            /** To Port */
            to_port: string;
            /** Value */
            value?: unknown | null;
            /**
             * Conditional
             * @default false
             */
            conditional: boolean;
        };
        /** PyGraph */
        PyGraph: {
            /** Nodes */
            nodes: components["schemas"]["PyNode"][];
            /** Edges */
            edges: components["schemas"]["PyEdge"][];
        };
        /** PyNode */
        PyNode: {
            /** Id */
            id: string;
            /**
             * Status
             * @enum {string}
             */
            status: "Not started" | "Started" | "Error" | "Finished";
            /** Function Name */
            function_name: string;
            /**
             * Node Location
             * @default
             */
            node_location: string;
            /** Value */
            value?: unknown | null;
            /** Started Time */
            started_time: string;
            /** Finished Time */
            finished_time: string;
        };
        /** ValidationError */
        ValidationError: {
            /** Location */
            loc: (string | number)[];
            /** Message */
            msg: string;
            /** Error Type */
            type: string;
        };
        /** WorkflowDisplay */
        WorkflowDisplay: {
            /**
             * Id
             * Format: uuid
             */
            id: string;
            /** Id Int */
            id_int: number;
            /** Name */
            name: string | null;
            /** Start Time */
            start_time: string;
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
    list_workflows_api_workflows__get: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
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
    list_nodes_api_workflows__workflow_id__graphs_get: {
        parameters: {
            query: {
                locs: string[];
            };
            header?: never;
            path: {
                workflow_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GraphsResponse"];
                };
            };
            /** @description Validation Error */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HTTPValidationError"];
                };
            };
        };
    };
    get_node_api_workflows__workflow_id__nodes__node_location_str__get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                workflow_id: string;
                node_location_str: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Func"] | components["schemas"]["Eval"] | components["schemas"]["Loop"] | components["schemas"]["Map"] | components["schemas"]["Const"] | components["schemas"]["IfElse"] | components["schemas"]["EagerIfElse"] | components["schemas"]["Input"] | components["schemas"]["Output"];
                };
            };
            /** @description Validation Error */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HTTPValidationError"];
                };
            };
        };
    };
    get_input_api_workflows__workflow_id__nodes__node_location_str__inputs__port_name__get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                workflow_id: string;
                node_location_str: string;
                port_name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description Validation Error */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HTTPValidationError"];
                };
            };
        };
    };
    get_output_api_workflows__workflow_id__nodes__node_location_str__outputs__port_name__get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                workflow_id: string;
                node_location_str: string;
                port_name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description Validation Error */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HTTPValidationError"];
                };
            };
        };
    };
    get_logs_api_workflows__workflow_id__logs_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                workflow_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description Validation Error */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HTTPValidationError"];
                };
            };
        };
    };
    get_errors_api_workflows__workflow_id__nodes__node_location_str__errors_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                workflow_id: string;
                node_location_str: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description Validation Error */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HTTPValidationError"];
                };
            };
        };
    };
    read_root__path__get: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
        };
    };
}
