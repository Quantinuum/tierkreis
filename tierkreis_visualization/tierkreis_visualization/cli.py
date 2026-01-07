import argparse


from tierkreis_visualization.main import start, dev, graph


def parse_args(
    parser: argparse.ArgumentParser,
) -> argparse.ArgumentParser:
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        "--dev",
        help="Set this flag to run the server in development mode, including hot reload on change of server code.",
        action="store_true",
    )
    group.add_argument(
        "--graph",
        help="Visualizes a graph data object of the form <PATH_TO_PYTHON_FILE>:<VARIABLE_CONTAINING_GRAPH>",
        type=str,
    )
    return parser


def run_args(args: argparse.Namespace) -> None:
    if args.dev:
        dev()
    elif args.graph:
        graph(3)
    else:
        start()


class TierkreisVizCli:
    @staticmethod
    def add_subcommand(
        main_parser: argparse._SubParsersAction,
    ) -> None:
        parser: argparse.ArgumentParser = main_parser.add_parser(
            "vis",
            description="Runs the tierkreis visualization.",
            help="Runs the tierkreis visualization.",
        )
        parser = parse_args(parser)
        parser.set_defaults(func=TierkreisVizCli.execute)

    @staticmethod
    def execute(args: argparse.Namespace) -> None:
        run_args(args)
