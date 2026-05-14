pub mod auth {
    tonic::include_proto!("agentspay.auth.v1");
}

pub mod common {
    tonic::include_proto!("agentspay.common.v1");
}

pub mod metering {
    tonic::include_proto!("agentspay.metering.v1");
}

pub mod payment {
    tonic::include_proto!("agentspay.payment.v1");
}
