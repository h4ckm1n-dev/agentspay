from __future__ import annotations

import json
import uuid
from dataclasses import asdict
from typing import Mapping
from urllib.error import HTTPError
from urllib.request import Request, urlopen

from .errors import AgentsPayApiError, AgentsPayError
from .types import (
    AgentsPayConfig,
    JsonObject,
    PayAndCallRequest,
    PayAndCallResult,
    PaymentAuthorization,
    PaymentRequirement,
    PaymentRequirementRequest,
    PaymentSettlement,
    PaymentVerification,
)


class AgentsPayClient:
    def __init__(
        self,
        config: AgentsPayConfig | None = None,
        *,
        base_url: str | None = None,
        api_key: str | None = None,
        environment: str | None = None,
        timeout: float | None = None,
        debug: bool | None = None,
    ) -> None:
        selected = config or AgentsPayConfig()
        self.config = AgentsPayConfig(
            base_url=(base_url or selected.base_url).rstrip("/"),
            api_key=api_key if api_key is not None else selected.api_key,
            environment=environment if environment in {"sandbox", "live"} else selected.environment,
            timeout=timeout if timeout is not None else selected.timeout,
            debug=debug if debug is not None else selected.debug,
        )

    def health(self) -> JsonObject:
        return self._request("GET", "/v1/health")

    def status(self) -> JsonObject:
        return self._request("GET", "/v1/status")

    def create_payment_requirement(
        self,
        requirement: PaymentRequirementRequest,
    ) -> PaymentRequirement:
        idempotency_key = requirement.idempotency_key or create_idempotency_key()
        body: JsonObject = {
            "amount": requirement.amount,
            "currency": requirement.currency,
            "idempotency_key": idempotency_key,
        }
        optional_values: Mapping[str, object | None] = {
            "endpoint_id": requirement.endpoint_id,
            "method": requirement.method,
            "path": requirement.path,
            "url": requirement.url,
            "description": requirement.description,
            "payer_agent_id": requirement.payer_agent_id,
            "metadata": requirement.metadata,
        }
        body.update({key: value for key, value in optional_values.items() if value is not None})

        payload = self._request(
            "POST",
            "/v1/payment-requirements",
            body=body,
            idempotency_key=idempotency_key,
        )
        return _payment_requirement_from_payload(
            payload,
            amount=requirement.amount,
            currency=requirement.currency,
        )

    def authorize_payment(
        self,
        requirement: PaymentRequirement,
        *,
        max_amount: str | None = None,
        payer_agent_id: str | None = None,
        metadata: JsonObject | None = None,
        idempotency_key: str | None = None,
    ) -> PaymentAuthorization:
        key = idempotency_key or create_idempotency_key()
        body: JsonObject = {
            "payment_requirement": _dataclass_payload(requirement),
            "idempotency_key": key,
        }
        if max_amount is not None:
            body["max_amount"] = max_amount
        if payer_agent_id is not None:
            body["payer_agent_id"] = payer_agent_id
        if metadata is not None:
            body["metadata"] = metadata

        payload = self._request(
            "POST",
            "/v1/payments/authorize",
            body=body,
            idempotency_key=key,
        )
        return _authorization_from_payload(payload, requirement_id=requirement.id, key=key)

    def verify_payment(
        self,
        requirement: PaymentRequirement,
        authorization: PaymentAuthorization,
        *,
        metadata: JsonObject | None = None,
        idempotency_key: str | None = None,
    ) -> PaymentVerification:
        key = idempotency_key or create_idempotency_key()
        body: JsonObject = {
            "payment_requirement": _dataclass_payload(requirement),
            "authorization": _dataclass_payload(authorization),
            "idempotency_key": key,
        }
        if metadata is not None:
            body["metadata"] = metadata

        payload = self._request(
            "POST",
            "/v1/payments/verify",
            body=body,
            idempotency_key=key,
        )
        return _verification_from_payload(payload)

    def settle_payment(
        self,
        authorization: PaymentAuthorization,
        *,
        requirement: PaymentRequirement | None = None,
        verification: PaymentVerification | None = None,
        metadata: JsonObject | None = None,
        idempotency_key: str | None = None,
    ) -> PaymentSettlement:
        key = idempotency_key or create_idempotency_key()
        body: JsonObject = {
            "authorization": _dataclass_payload(authorization),
            "idempotency_key": key,
        }
        if requirement is not None:
            body["payment_requirement"] = _dataclass_payload(requirement)
        if verification is not None:
            body["verification"] = _dataclass_payload(verification)
        if metadata is not None:
            body["metadata"] = metadata

        payload = self._request(
            "POST",
            "/v1/payments/settle",
            body=body,
            idempotency_key=key,
        )
        return _settlement_from_payload(payload)

    def pay_and_call(self, call: PayAndCallRequest) -> PayAndCallResult:
        status, headers, body = self._raw_request(
            call.method,
            call.url,
            headers=call.headers,
            body=call.body,
            absolute_url=True,
        )

        if status != 402 or not call.retry_on_402:
            return PayAndCallResult(
                status=status,
                headers=headers,
                body=body,
                payment_required=status == 402,
            )

        requirement = call.payment_requirement or _extract_requirement_from_402(call, body, headers)
        self._log(f"received 402 payment requirement {requirement.id}")
        authorization = self.authorize_payment(
            requirement,
            max_amount=call.max_amount,
            idempotency_key=call.idempotency_key,
        )

        retry_headers = dict(call.headers or {})
        payment_signature = (
            authorization.payment_signature
            or authorization.payment_header
            or authorization.id
        )
        retry_headers["PAYMENT-SIGNATURE"] = payment_signature
        retry_headers["PAYMENT-RESPONSE"] = json.dumps(_dataclass_payload(authorization))
        retry_headers["X-AgentsPay-Authorization"] = authorization.id

        retry_status, retry_response_headers, retry_body = self._raw_request(
            call.method,
            call.url,
            headers=retry_headers,
            body=call.body,
            absolute_url=True,
        )

        if retry_status == 402:
            raise AgentsPayError(
                "Payment retry was rejected by the paid endpoint.",
                code="payment_retry_rejected",
                status=retry_status,
                details=_object_details(retry_body),
            )

        settlement = (
            self.settle_payment(authorization, requirement=requirement)
            if call.settle
            else None
        )
        return PayAndCallResult(
            status=retry_status,
            headers=retry_response_headers,
            body=retry_body,
            payment_required=True,
            requirement=requirement,
            authorization=authorization,
            settlement=settlement,
        )

    def _request(
        self,
        method: str,
        path: str,
        *,
        body: JsonObject | None = None,
        idempotency_key: str | None = None,
    ) -> JsonObject:
        status, _, payload = self._raw_request(
            method,
            path,
            headers={"Idempotency-Key": idempotency_key} if idempotency_key else None,
            body=body,
            absolute_url=False,
        )

        if status < 200 or status >= 300:
            raise AgentsPayApiError(
                "AgentsPay API request failed.",
                status=status,
                details=_object_details(payload),
            )

        if isinstance(payload, dict):
            return payload
        return {"data": payload}

    def _raw_request(
        self,
        method: str,
        target: str,
        *,
        headers: Mapping[str, str] | None = None,
        body: object | None = None,
        absolute_url: bool,
    ) -> tuple[int, dict[str, str], object | None]:
        url = target if absolute_url else f"{self.config.base_url}{target}"
        request_headers = self._headers(headers)
        data = _encode_body(body, request_headers)
        request = Request(url, data=data, headers=request_headers, method=method.upper())

        self._log(f"{method.upper()} {url}")
        try:
            with urlopen(request, timeout=self.config.timeout) as response:
                response_body = response.read()
                return (
                    response.status,
                    dict(response.headers.items()),
                    _decode_body(response_body, response.headers.get("content-type")),
                )
        except HTTPError as error:
            error_body = error.read()
            return (
                error.code,
                dict(error.headers.items()),
                _decode_body(error_body, error.headers.get("content-type")),
            )

    def _headers(self, extra: Mapping[str, str] | None = None) -> dict[str, str]:
        headers = {
            "Accept": "application/json",
            "X-AgentsPay-Environment": self.config.environment,
        }
        if self.config.api_key:
            headers["Authorization"] = f"Bearer {self.config.api_key}"
        if extra:
            headers.update({key: value for key, value in extra.items() if value is not None})
        return headers

    def _log(self, message: str) -> None:
        if self.config.debug:
            print(f"[agentspay] {message}")


def create_idempotency_key(prefix: str = "ap") -> str:
    return f"{prefix}_{uuid.uuid4()}"


def _encode_body(body: object | None, headers: dict[str, str]) -> bytes | None:
    if body is None:
        return None
    if isinstance(body, bytes):
        return body
    if isinstance(body, str):
        return body.encode("utf-8")
    headers["Content-Type"] = "application/json"
    return json.dumps(body).encode("utf-8")


def _decode_body(body: bytes, content_type: str | None) -> object | None:
    if not body:
        return None
    text = body.decode("utf-8")
    if content_type and "application/json" in content_type:
        return json.loads(text)
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return text


def _payment_requirement_from_payload(
    payload: Mapping[str, object],
    *,
    amount: str,
    currency: str,
) -> PaymentRequirement:
    return PaymentRequirement(
        id=_string(payload.get("id")) or create_idempotency_key("req"),
        amount=_string(payload.get("amount")) or amount,
        currency=_string(payload.get("currency")) or currency,
        endpoint_id=_string(payload.get("endpoint_id")),
        description=_string(payload.get("description")),
        expires_at=_string(payload.get("expires_at")),
        payment_url=_string(payload.get("payment_url")),
        raw=dict(payload),
    )


def _authorization_from_payload(
    payload: Mapping[str, object],
    *,
    requirement_id: str,
    key: str,
) -> PaymentAuthorization:
    return PaymentAuthorization(
        id=_string(payload.get("id")) or create_idempotency_key("auth"),
        requirement_id=_string(payload.get("requirement_id")) or requirement_id,
        status=_string(payload.get("status")) or "authorized",
        payment_signature=_string(payload.get("payment_signature")),
        payment_header=_string(payload.get("payment_header")),
        expires_at=_string(payload.get("expires_at")),
        idempotency_key=key,
        raw=dict(payload),
    )


def _verification_from_payload(payload: Mapping[str, object]) -> PaymentVerification:
    accepted = _bool(payload.get("accepted"))
    if accepted is None:
        accepted = _bool(payload.get("valid")) or False

    return PaymentVerification(
        accepted=accepted,
        id=_string(payload.get("id")),
        status=_string(payload.get("status")),
        reason=_string(payload.get("reason")),
        raw=dict(payload),
    )


def _settlement_from_payload(payload: Mapping[str, object]) -> PaymentSettlement:
    return PaymentSettlement(
        id=_string(payload.get("id")),
        status=_string(payload.get("status")) or "settled",
        transaction_id=_string(payload.get("transaction_id")),
        audit_proof_id=_string(payload.get("audit_proof_id")),
        raw=dict(payload),
    )


def _extract_requirement_from_402(
    call: PayAndCallRequest,
    body: object | None,
    headers: Mapping[str, str],
) -> PaymentRequirement:
    payload = body if isinstance(body, dict) else {}
    direct = payload.get("payment_requirement") or payload.get("paymentRequirement")
    if isinstance(direct, dict):
        payload = direct
    else:
        accepts = payload.get("accepts")
        requirements = payload.get("requirements")
        if isinstance(accepts, list) and accepts and isinstance(accepts[0], dict):
            payload = accepts[0]
        elif isinstance(requirements, list) and requirements and isinstance(requirements[0], dict):
            payload = requirements[0]

    header_id = headers.get("PAYMENT-REQUIRED") or headers.get(
        "X-AgentsPay-Payment-Requirement-Id"
    )
    return PaymentRequirement(
        id=_string(payload.get("id")) or header_id or create_idempotency_key("req"),
        amount=_string(payload.get("amount")) or call.max_amount or "0",
        currency=_string(payload.get("currency")) or call.currency,
        endpoint_id=_string(payload.get("endpoint_id")) or call.endpoint_id,
        description=_string(payload.get("description")) or call.description,
        expires_at=_string(payload.get("expires_at")),
        payment_url=_string(payload.get("payment_url")),
        raw=dict(payload),
    )


def _dataclass_payload(value: object) -> JsonObject:
    payload = asdict(value)
    return {key: item for key, item in payload.items() if item is not None}


def _object_details(value: object | None) -> Mapping[str, object] | None:
    if isinstance(value, dict):
        return value
    if value is None:
        return None
    return {"data": value}


def _string(value: object) -> str | None:
    return value if isinstance(value, str) else None


def _bool(value: object) -> bool | None:
    return value if isinstance(value, bool) else None
