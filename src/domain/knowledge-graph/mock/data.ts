import type { GraphEntity } from "@/domain/knowledge-graph/model/model";

export function getGraphData(domain = "customer-support") {
  const domainLabel = domain === "supply-chain" ? "Supply Chain" : domain === "hr" ? "HR" : "Customer Support";
  const entities: GraphEntity[] = [
    {
      id: "customer-0192",
      label: domain === "hr" ? "Employee_0192" : "Customer_0192",
      type: domain === "hr" ? "Employee Profile" : "User Profile",
      x: 350,
      y: 250,
      properties: { status: "Active", created_at: "2023-10-24T08:12:00Z", ltv_segment: "High Value", domain: domainLabel },
      relationships: [
        { label: domain === "supply-chain" ? "PLACED_ORDER" : "MADE_PURCHASE", target: "Order_TXN_55" },
        { label: domain === "hr" ? "OWNS_DEVICE" : "MADE_PURCHASE", target: "Order_TXN_56" },
      ],
    },
    {
      id: "order-55",
      label: "Order_TXN_55",
      type: "Transaction",
      x: 500,
      y: 400,
      properties: { status: "Fulfilled", created_at: "2023-10-25T11:09:00Z", total: "$284.10", domain: domainLabel },
      relationships: [{ label: "CONTAINS", target: "Product_SKU_A1" }],
    },
    {
      id: "sku-a1",
      label: "Product_SKU_A1",
      type: "Inventory Item",
      x: 650,
      y: 300,
      properties: { status: "In Stock", created_at: "2023-09-12T03:20:00Z", tier: "Priority", domain: domainLabel },
      relationships: [],
    },
    {
      id: "order-56",
      label: "Order_TXN_56",
      type: "Transaction",
      x: 200,
      y: 400,
      properties: { status: "Review", created_at: "2023-10-26T15:40:00Z", total: "$92.00", domain: domainLabel },
      relationships: [],
    },
  ];

  return { domainLabel, entities };
}
