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
        help="Visualizes a graph data object of the form module_path:graph",
        type=str,
    )
    return parser


def run_args(args: argparse.Namespace) -> None:
    if args.dev:
        dev()
    elif args.graph:
        graph()
    else:
        start()  # check if sysarg works as expected


class TierkreisVizCli:
    @staticmethod
    def add_subcommand(
        main_parser: argparse._SubParsersAction[argparse.ArgumentParser],
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
