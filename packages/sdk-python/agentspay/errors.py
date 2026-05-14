from __future__ import annotations

from dataclasses import dataclass
from typing import Mapping


@dataclass(frozen=True)
class AgentsPayError(Exception):
    message: str
    code: str = "agentspay_error"
    status: int | None = None
    details: Mapping[str, object] | None = None

    def __str__(self) -> str:
        if self.status is None:
            return self.message
        return f"{self.message} (status={self.status})"


class AgentsPayApiError(AgentsPayError):
    def __init__(
        self,
        message: str,
        *,
        status: int | None = None,
        details: Mapping[str, object] | None = None,
    ) -> None:
        super().__init__(
            message=message,
            code="agentspay_api_error",
            status=status,
            details=details,
        )
