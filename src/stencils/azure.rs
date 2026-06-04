//! Azure (mxgraph.azure) stencil entries.

use super::Entry;

pub const ENTRIES: &[Entry] = &[
    Entry { key: "azure.vm", path: "mxgraph.azure.virtual_machine", desc: "Virtual machine" },
    Entry { key: "azure.vm_scale_set", path: "mxgraph.azure.virtual_machine_scale_sets", desc: "VM scale set" },
    Entry { key: "azure.app_service", path: "mxgraph.azure.app_services", desc: "App Service" },
    Entry { key: "azure.functions", path: "mxgraph.azure.function_apps", desc: "Function app" },
    Entry { key: "azure.aks", path: "mxgraph.azure.kubernetes_services", desc: "AKS" },
    Entry { key: "azure.container_instances", path: "mxgraph.azure.container_instances", desc: "Container Instances" },
    Entry { key: "azure.sql", path: "mxgraph.azure.sql_database", desc: "SQL Database" },
    Entry { key: "azure.cosmos_db", path: "mxgraph.azure.azure_cosmos_db", desc: "Cosmos DB" },
    Entry { key: "azure.storage", path: "mxgraph.azure.storage_accounts", desc: "Storage account" },
    Entry { key: "azure.blob", path: "mxgraph.azure.blob_storage", desc: "Blob storage" },
    Entry { key: "azure.vnet", path: "mxgraph.azure.virtual_networks", desc: "Virtual network" },
    Entry { key: "azure.load_balancer", path: "mxgraph.azure.load_balancers", desc: "Load balancer" },
    Entry { key: "azure.app_gateway", path: "mxgraph.azure.application_gateway", desc: "Application Gateway" },
    Entry { key: "azure.firewall", path: "mxgraph.azure.firewall", desc: "Firewall" },
    Entry { key: "azure.cdn", path: "mxgraph.azure.cdn_profiles", desc: "CDN" },
    Entry { key: "azure.service_bus", path: "mxgraph.azure.azure_service_bus", desc: "Service Bus" },
    Entry { key: "azure.event_hub", path: "mxgraph.azure.event_hubs", desc: "Event Hubs" },
    Entry { key: "azure.key_vault", path: "mxgraph.azure.key_vaults", desc: "Key Vault" },
    Entry { key: "azure.active_directory", path: "mxgraph.azure.azure_active_directory", desc: "Active Directory" },
    Entry { key: "azure.monitor", path: "mxgraph.azure.monitor", desc: "Monitor" },
];
