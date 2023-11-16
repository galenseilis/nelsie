import re

from .layoutexpr import XExpr, ConstExpr, YExpr
from .basictypes import Size
from .export import ExportSize, PointsSize, FractionSize, AUTO_SIZE

SIZE_REGEXP = re.compile(r"^(\d+(?:\.\d+)?)(%)?$")


def parse_size(value: Size) -> ExportSize:
    if value == "auto":
        return AUTO_SIZE
    if isinstance(value, (int, float)):
        return PointsSize(float(value))
    if isinstance(value, str):
        match = SIZE_REGEXP.match(value)
        if match:
            number, percent = match.groups()
            return (
                FractionSize(float(number) / 100.0)
                if percent
                else PointsSize(float(number))
            )
    raise ValueError(f"Invalid size definition: {value!r}")


def parse_position(parent_id, obj, is_x):
    if obj is None:
        return None
    if isinstance(obj, (int, float)):
        if is_x:
            return XExpr(parent_id) + ConstExpr(obj)
        else:
            return YExpr(parent_id) + ConstExpr(obj)
    raise ValueError("Invalid position")


def parse_position_y(parent_id, obj):
    if obj is None:
        return None
    if isinstance(obj, (int, float)):
        return XExpr(parent_id) + ConstExpr(obj)
    raise ValueError("Invalid position")


def check_type(obj, cls):
    if isinstance(obj, cls):
        return obj
    raise TypeError(f"Expected {cls} got {type(obj)}")


def check_type_bool(obj):
    return check_type(obj, bool)
