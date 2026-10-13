"""Versioned interface between generated HTTP artifacts and their runtime.

Generated packages use format version 1 and pass it to ``install_api``. These
constructors describe dispatch already checked by codegen; application code uses
``jsoncompat.http`` instead. Keep runtime implementation details behind this
module so generated source need not depend on private binding names.
"""

from jsoncompat.http._binding import (
    _Field as Field,
    _Operation as OperationPlan,
    _ResponsePlan as ResponsePlan,
    install_generated_api as install_api,
)

__all__ = ("Field", "OperationPlan", "ResponsePlan", "install_api")
