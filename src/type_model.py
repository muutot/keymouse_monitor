import typing

MonitorT = typing.Any

if typing.TYPE_CHECKING:
    from src.monitor import Monitor

    MonitorT = Monitor
