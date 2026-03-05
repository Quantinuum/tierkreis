from sys import argv

from tkr_pytket_worker_impl import worker


def main():
    worker.app(argv)


if __name__ == "__main__":
    main()
