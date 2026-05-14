from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal, TypeAlias

AgentsPayEnvironment: TypeAlias = Literal["sandbox", "live"]
JsonPrimitive: TypeAlias = str | int | float | bool | None
JsonObject: TypeAlias = dict[str, object]


@dataclass(frozen=True)
class AgentsPayConfig:
    base_url: str = "http://localhost:8080"
    api_key: str | None = None
    environment: AgentsPayEnvironment = "sandbox"
    timeout: float = 30.0
    debug: bool = False


@dataclass(frozen=True)
class PaymentRequirementRequest:
    amount: str
    currency: str = "USDC"
    endpoint_id: str | None = None
    method: str | None = None
    path: str | None = None
    url: str | None = None
    description: str | None = None
    payer_agent_id: str | None = None
    metadata: JsonObject | None = None
    idempotency_key: str | None = None


@dataclass(frozen=True)
class PaymentRequirement:
    id: str
    amount: str
    currency: str
    endpoint_id: str | None = None
    description: str | None = None
    expires_at: str | None = None
    payment_url: str | None = None
    raw: JsonObject = field(default_factory=dict)


@dataclass(frozen=True)
class PaymentAuthorization:
    id: str
    requirement_id: str | None = None
    status: str | None = None
    payment_signature: str | None = None
    payment_header: str | None = None
    expires_at: str | None = None
    idempotency_key: str | None = None
    raw: JsonObject = field(default_factory=dict)


@dataclass(frozen=True)
class PaymentVerification:
    accepted: bool
    id: str | None = None
    status: str | None = None
    reason: str | None = None
    raw: JsonObject = field(default_factory=dict)


@dataclass(frozen=True)
class PaymentSettlement:
    status: str
    id: str | None = None
    transaction_id: str | None = None
    audit_proof_id: str | None = None
    raw: JsonObject = field(default_factory=dict)


@dataclass(frozen=True)
class PayAndCallRequest:
    url: str
    method: str = "POST"
    headers: dict[str, str] | None = None
    body: JsonObject | list[object] | str | bytes | None = None
    max_amount: str | None = None
    currency: str = "USDC"
    endpoint_id: str | None = None
    description: str | None = None
    payment_requirement: PaymentRequirement | None = None
    idempotency_key: str | None = None
    retry_on_402: bool = True
    settle: bool = False


@dataclass(frozen=True)
class PayAndCallResult:
    status: int
    headers: dict[str, str]
    body: object | None
    payment_required: bool
    requirement: PaymentRequirement | None = None
    authorization: PaymentAuthorization | None = None
    settlement: PaymentSettlement | None = None
