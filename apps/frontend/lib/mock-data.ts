export type Environment = "sandbox" | "live";

export type HealthState = "operational" | "degraded" | "blocked";

export interface BalanceSnapshot {
  environment: Environment;
  available: number;
  reserved: number;
  settledToday: number;
  pendingSettlement: number;
  dailyLimit: number;
}

export interface Agent {
  id: string;
  name: string;
  owner: string;
  status: "active" | "paused" | "review";
  dailyLimit: number;
  spentToday: number;
  maxPerRequest: number;
  allowedEndpoints: number;
}

export interface PaidEndpoint {
  id: string;
  name: string;
  method: "GET" | "POST";
  path: string;
  price: number;
  currency: "USDC";
  requestsToday: number;
  authorizationRate: number;
  status: "live" | "sandbox" | "paused";
}

export interface Transaction {
  id: string;
  agent: string;
  endpoint: string;
  amount: number;
  status: "authorized" | "settled" | "failed" | "refunded";
  policyDecision: "approved" | "denied";
  auditProof: string;
  occurredAt: string;
}

export interface PolicyDecision {
  id: string;
  agent: string;
  rule: string;
  result: "approved" | "denied" | "review";
  reason: string;
  occurredAt: string;
}

export interface FacilitatorCheck {
  name: string;
  path: string;
  state: HealthState;
  latencyMs: number;
  detail: string;
}

export interface WebhookDelivery {
  id: string;
  target: string;
  event: string;
  state: "delivered" | "retrying" | "failed";
  attempts: number;
}

export interface AuditProof {
  id: string;
  subject: string;
  state: "sealed" | "pending";
  decisionCount: number;
}

export const balances: Record<Environment, BalanceSnapshot> = {
  sandbox: {
    environment: "sandbox",
    available: 12450,
    reserved: 380,
    settledToday: 2187,
    pendingSettlement: 142,
    dailyLimit: 20000
  },
  live: {
    environment: "live",
    available: 1840,
    reserved: 118,
    settledToday: 427,
    pendingSettlement: 82,
    dailyLimit: 3000
  }
};

export const agents: Record<Environment, Agent[]> = {
  sandbox: [
    {
      id: "agt_research_01",
      name: "Research Analyst",
      owner: "Platform",
      status: "active",
      dailyLimit: 4000,
      spentToday: 1140,
      maxPerRequest: 35,
      allowedEndpoints: 9
    },
    {
      id: "agt_eval_02",
      name: "Eval Runner",
      owner: "ML Ops",
      status: "active",
      dailyLimit: 2500,
      spentToday: 640,
      maxPerRequest: 20,
      allowedEndpoints: 6
    },
    {
      id: "agt_scrape_03",
      name: "Data Collector",
      owner: "Growth",
      status: "review",
      dailyLimit: 1200,
      spentToday: 1030,
      maxPerRequest: 8,
      allowedEndpoints: 4
    }
  ],
  live: [
    {
      id: "agt_prod_01",
      name: "Production Buyer",
      owner: "Platform",
      status: "active",
      dailyLimit: 900,
      spentToday: 244,
      maxPerRequest: 15,
      allowedEndpoints: 3
    },
    {
      id: "agt_support_02",
      name: "Support Resolver",
      owner: "Customer Ops",
      status: "paused",
      dailyLimit: 350,
      spentToday: 0,
      maxPerRequest: 5,
      allowedEndpoints: 2
    }
  ]
};

export const paidEndpoints: Record<Environment, PaidEndpoint[]> = {
  sandbox: [
    {
      id: "endp_search",
      name: "Premium Search",
      method: "POST",
      path: "/v1/search/enriched",
      price: 0.018,
      currency: "USDC",
      requestsToday: 18420,
      authorizationRate: 0.982,
      status: "sandbox"
    },
    {
      id: "endp_summarize",
      name: "Long Context Summary",
      method: "POST",
      path: "/v1/summarize",
      price: 0.042,
      currency: "USDC",
      requestsToday: 7240,
      authorizationRate: 0.967,
      status: "sandbox"
    },
    {
      id: "endp_verify",
      name: "Identity Verification",
      method: "POST",
      path: "/v1/verify/account",
      price: 0.12,
      currency: "USDC",
      requestsToday: 980,
      authorizationRate: 0.941,
      status: "paused"
    }
  ],
  live: [
    {
      id: "endp_search_live",
      name: "Premium Search",
      method: "POST",
      path: "/v1/search/enriched",
      price: 0.018,
      currency: "USDC",
      requestsToday: 1280,
      authorizationRate: 0.991,
      status: "live"
    },
    {
      id: "endp_summary_live",
      name: "Long Context Summary",
      method: "POST",
      path: "/v1/summarize",
      price: 0.042,
      currency: "USDC",
      requestsToday: 512,
      authorizationRate: 0.978,
      status: "live"
    }
  ]
};

export const transactions: Record<Environment, Transaction[]> = {
  sandbox: [
    {
      id: "txn_7Z9C2",
      agent: "Research Analyst",
      endpoint: "Premium Search",
      amount: 0.018,
      status: "settled",
      policyDecision: "approved",
      auditProof: "audit_9f12",
      occurredAt: "00:22:14"
    },
    {
      id: "txn_7Z9B8",
      agent: "Data Collector",
      endpoint: "Identity Verification",
      amount: 0.12,
      status: "failed",
      policyDecision: "denied",
      auditProof: "audit_9ef0",
      occurredAt: "00:20:03"
    },
    {
      id: "txn_7Z984",
      agent: "Eval Runner",
      endpoint: "Long Context Summary",
      amount: 0.042,
      status: "authorized",
      policyDecision: "approved",
      auditProof: "audit_9ea4",
      occurredAt: "00:18:49"
    }
  ],
  live: [
    {
      id: "txn_live_311",
      agent: "Production Buyer",
      endpoint: "Premium Search",
      amount: 0.018,
      status: "settled",
      policyDecision: "approved",
      auditProof: "audit_live_72",
      occurredAt: "00:17:44"
    },
    {
      id: "txn_live_310",
      agent: "Production Buyer",
      endpoint: "Long Context Summary",
      amount: 0.042,
      status: "authorized",
      policyDecision: "approved",
      auditProof: "audit_live_71",
      occurredAt: "00:14:12"
    }
  ]
};

export const policyDecisions: Record<Environment, PolicyDecision[]> = {
  sandbox: [
    {
      id: "pol_3029",
      agent: "Research Analyst",
      rule: "Daily budget",
      result: "approved",
      reason: "28% of budget used",
      occurredAt: "00:22:14"
    },
    {
      id: "pol_3028",
      agent: "Data Collector",
      rule: "Endpoint allowlist",
      result: "denied",
      reason: "Endpoint paused for review",
      occurredAt: "00:20:03"
    },
    {
      id: "pol_3027",
      agent: "Eval Runner",
      rule: "Max amount",
      result: "approved",
      reason: "0.042 USDC under 20 USDC limit",
      occurredAt: "00:18:49"
    }
  ],
  live: [
    {
      id: "pol_live_901",
      agent: "Production Buyer",
      rule: "Live access",
      result: "approved",
      reason: "Invite gate satisfied",
      occurredAt: "00:17:44"
    },
    {
      id: "pol_live_900",
      agent: "Support Resolver",
      rule: "Agent status",
      result: "review",
      reason: "Agent paused by owner",
      occurredAt: "00:12:01"
    }
  ]
};

export const facilitatorChecks: Record<Environment, FacilitatorCheck[]> = {
  sandbox: [
    {
      name: "Verify",
      path: "/x402/verify",
      state: "operational",
      latencyMs: 42,
      detail: "Payment proofs accepted"
    },
    {
      name: "Settle",
      path: "/x402/settle",
      state: "operational",
      latencyMs: 88,
      detail: "Sandbox ledger synced"
    },
    {
      name: "Supported",
      path: "/x402/supported",
      state: "operational",
      latencyMs: 21,
      detail: "Exact pricing advertised"
    }
  ],
  live: [
    {
      name: "Verify",
      path: "/x402/verify",
      state: "operational",
      latencyMs: 55,
      detail: "Live policy checks enabled"
    },
    {
      name: "Settle",
      path: "/x402/settle",
      state: "degraded",
      latencyMs: 240,
      detail: "Settlement queue above target"
    },
    {
      name: "Supported",
      path: "/x402/supported",
      state: "operational",
      latencyMs: 24,
      detail: "Live mode invite-only"
    }
  ]
};

export const webhookDeliveries: Record<Environment, WebhookDelivery[]> = {
  sandbox: [
    {
      id: "wh_001",
      target: "https://provider.test/webhooks",
      event: "transaction.settled",
      state: "delivered",
      attempts: 1
    },
    {
      id: "wh_002",
      target: "https://ops.test/events",
      event: "policy.denied",
      state: "retrying",
      attempts: 2
    }
  ],
  live: [
    {
      id: "wh_live_001",
      target: "https://api.example.com/webhooks",
      event: "transaction.authorized",
      state: "delivered",
      attempts: 1
    }
  ]
};

export const auditProofs: Record<Environment, AuditProof[]> = {
  sandbox: [
    {
      id: "audit_9f12",
      subject: "txn_7Z9C2",
      state: "sealed",
      decisionCount: 4
    },
    {
      id: "audit_9ef0",
      subject: "txn_7Z9B8",
      state: "sealed",
      decisionCount: 5
    }
  ],
  live: [
    {
      id: "audit_live_72",
      subject: "txn_live_311",
      state: "sealed",
      decisionCount: 4
    }
  ]
};
