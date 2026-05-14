# AgentsPay Python SDK

Minimal Python SDK scaffold for the AgentsPay MVP payment flow.

```python
from agentspay import AgentsPayClient, PaymentRequirementRequest

client = AgentsPayClient()
status = client.status()

requirement = client.create_payment_requirement(
    PaymentRequirementRequest(amount="0.002", currency="USDC")
)
authorization = client.authorize_payment(requirement)
```
