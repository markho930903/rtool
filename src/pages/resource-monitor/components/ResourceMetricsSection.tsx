import type { ResourceMetricCard } from "@/pages/resource-monitor/hooks/useResourceMonitorData";

interface ResourceMetricsSectionProps {
  metricCards: ResourceMetricCard[];
}

function MetricCard(props: ResourceMetricCard) {
  return (
    <article className="ui-glass-panel px-4 py-3">
      <div className="font-mono ui-text-micro uppercase tracking-ui-wide text-text-muted">{props.title}</div>
      <div className="mt-2 text-lg leading-none font-semibold text-text-primary">{props.value}</div>
      <div className="mt-1.5 text-xs text-text-secondary">{props.hint}</div>
    </article>
  );
}

export default function ResourceMetricsSection(props: ResourceMetricsSectionProps) {
  return (
    <section className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-4">
      {props.metricCards.map((item) => (
        <MetricCard key={item.title} title={item.title} value={item.value} hint={item.hint} />
      ))}
    </section>
  );
}
